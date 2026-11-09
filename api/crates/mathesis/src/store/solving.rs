use super::{MongoStore, from_document, unavailable};
use async_trait::async_trait;
use futures::TryStreamExt;
use mathesis_core::{
    CoreResult,
    model::{Entrant, ErrorReportFile, SolveFile},
    ports::SolvingStore,
};
use mongodb::{IndexModel, bson::doc, options::IndexOptions};
const REPORTS: &str = "error_reports";
const FILES: &str = "solve_files";
const AGENTS: &str = "agent_entrants";
pub(super) async fn ensure_indexes(store: &MongoStore) -> anyhow::Result<()> {
    let unique = |keys| {
        IndexModel::builder()
            .keys(keys)
            .options(IndexOptions::builder().unique(true).build())
            .build()
    };
    store
        .raw(REPORTS)
        .create_index(unique(doc! {"id": 1}))
        .await?;
    store
        .raw(FILES)
        .create_index(unique(doc! {"id": 1}))
        .await?;
    store
        .raw(FILES)
        .create_index(
            IndexModel::builder()
                .keys(doc! {"admitted": 1, "id": -1})
                .build(),
        )
        .await?;
    store
        .raw(AGENTS)
        .create_indexes([unique(doc! {"id": 1}), unique(doc! {"name": 1})])
        .await?;
    Ok(())
}
#[async_trait]
impl SolvingStore for MongoStore {
    async fn list_formalization_candidates(
        &self,
        limit: u32,
        before: Option<String>,
        include_preparing: bool,
    ) -> CoreResult<mathesis_core::ports::Listing<SolveFile>> {
        let filter = if include_preparing {
            doc! {"$or": [{"admitted": true}, {"collected": {"$type": "object"}}]}
        } else {
            doc! {"admitted": true}
        };
        self.page(FILES, filter, limit, before).await
    }
    async fn report_files(&self) -> CoreResult<Vec<ErrorReportFile>> {
        self.raw(REPORTS)
            .find(doc! {})
            .await
            .map_err(unavailable)?
            .try_collect::<Vec<_>>()
            .await
            .map_err(unavailable)?
            .into_iter()
            .map(from_document)
            .collect()
    }
    async fn insert_report_file(&self, file: &ErrorReportFile) -> CoreResult<()> {
        self.insert(REPORTS, file).await
    }
    async fn replace_report_file(&self, file: &ErrorReportFile, expected: u64) -> CoreResult<()> {
        self.replace_revision(REPORTS, "report file", &file.id, file, expected)
            .await
    }

    async fn get(&self, id: &str) -> CoreResult<Option<SolveFile>> {
        self.get_by_id(FILES, id).await
    }
    async fn all(&self) -> CoreResult<Vec<SolveFile>> {
        self.raw(FILES)
            .find(doc! {})
            .await
            .map_err(unavailable)?
            .try_collect::<Vec<_>>()
            .await
            .map_err(unavailable)?
            .into_iter()
            .map(from_document)
            .collect()
    }
    async fn insert(&self, file: &SolveFile) -> CoreResult<()> {
        self.insert(FILES, file).await
    }
    async fn replace(&self, file: &SolveFile, expected: u64) -> CoreResult<()> {
        self.replace_revision(FILES, "solve file", &file.id, file, expected)
            .await
    }
    async fn agents(&self) -> CoreResult<Vec<Entrant>> {
        self.raw(AGENTS)
            .find(doc! {})
            .await
            .map_err(unavailable)?
            .try_collect::<Vec<_>>()
            .await
            .map_err(unavailable)?
            .into_iter()
            .map(from_document)
            .collect()
    }
    async fn insert_agent(&self, agent: &Entrant) -> CoreResult<()> {
        self.insert(AGENTS, agent).await
    }
    async fn replace_agent(&self, agent: &Entrant, expected: u64) -> CoreResult<()> {
        self.replace_revision(AGENTS, "agent", &agent.id, agent, expected)
            .await
    }
}
