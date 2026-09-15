use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleWriteView {
    name: String,
    description: String,
    can_be_deleted: Option<bool>,
}

impl RoleWriteView {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub const fn can_be_deleted(&self) -> Option<bool> {
        self.can_be_deleted
    }
}
