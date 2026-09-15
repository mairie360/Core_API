use serde::Deserialize;
use utoipa::ToSchema;

pub const MAX_GROUP_NAME_LENGTH: usize = 255;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchGroupView {
    name: Option<String>,
    description: Option<String>,
}

impl PatchGroupView {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
