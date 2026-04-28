use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleWriteView {
    name: String,
    description: String,
    can_be_deleted: Option<bool>,
}

impl RoleWriteView {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn can_be_deleted(&self) -> Option<bool> {
        self.can_be_deleted
    }
}
