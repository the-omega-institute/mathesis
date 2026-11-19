use super::App;
use crate::{
    CoreResult,
    model::{Caller, Role},
    ports::ProcessingLane,
};

impl App {
    pub async fn processing_status(&self, caller: &Caller) -> CoreResult<Vec<ProcessingLane>> {
        caller.require(Role::Admin)?;
        self.ports.queue.processing_status().await
    }
}
