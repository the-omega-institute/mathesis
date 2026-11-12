use std::collections::BTreeMap;

use super::App;
use crate::{CoreError, CoreResult, ids::RecordId, model::*};

impl App {
    /// A successful public index replaces the sidecar, including removals. A
    /// failed parse/read/write leaves it intact. No review or proof work is queued.
    pub async fn reconcile_materials_index(&self, bytes: &[u8]) -> CoreResult<usize> {
        let entries = parse_materials_index(bytes)?;
        let saved = self.ports.materials.load_materials().await?;
        let mut admitted = BTreeMap::new();
        for entry in entries {
            let public = match entry.kind {
                MaterialsKind::Collected => self
                    .ports
                    .collected
                    .get(&entry.id, Some(entry.materials.version))
                    .await?
                    .is_some(),
                MaterialsKind::Record => {
                    if let Some(record) =
                        self.ports.records.get(&RecordId(entry.id.clone())).await?
                    {
                        match self.accepted(&record).await {
                            Ok(s) => {
                                s.analysis_visibility == Visibility::Public
                                    && s.current_version()
                                        .is_some_and(|v| v.number == entry.materials.version)
                            }
                            Err(CoreError::NotFound { .. }) => false,
                            Err(e) => return Err(e),
                        }
                    } else {
                        false
                    }
                }
            };
            if public {
                admitted.insert(
                    (entry.kind, entry.id.clone(), entry.materials.version),
                    entry,
                );
            }
        }
        let entries: Vec<_> = admitted.into_values().collect();
        let count = entries.len();
        if saved.entries != entries {
            self.ports
                .materials
                .replace_materials(
                    &MaterialsSidecar {
                        revision: saved.revision + 1,
                        entries,
                    },
                    saved.revision,
                )
                .await?;
        }
        Ok(count)
    }

    pub(super) async fn public_materials(
        &self,
        kind: MaterialsKind,
        id: &str,
        version: u32,
    ) -> CoreResult<Option<PaperMaterials>> {
        Ok(self
            .ports
            .materials
            .load_materials()
            .await?
            .entries
            .into_iter()
            .find(|e| e.kind == kind && e.id == id && e.materials.version == version)
            .map(|e| e.materials))
    }
}
