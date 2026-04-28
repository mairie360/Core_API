use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchView {
    name: Option<String>,
    description: Option<String>,
    can_be_deleted: Option<Option<bool>>,
}

impl PatchView {
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn can_be_deleted(&self) -> Option<Option<bool>> {
        self.can_be_deleted.clone()
    }
}
