use crate::database::groups::get_group::Group;
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupResultView {
    group: Group,
}

impl GetGroupResultView {
    pub fn new(group: Group) -> Self {
        Self { group }
    }
}
