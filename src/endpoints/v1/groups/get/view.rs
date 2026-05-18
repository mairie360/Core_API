use crate::database::groups::get_group::Group;
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupsResultView {
    groups: Vec<Group>,
}

impl GetGroupsResultView {
    pub fn new(groups: Vec<Group>) -> Self {
        Self { groups }
    }
}

impl From<Vec<Group>> for GetGroupsResultView {
    fn from(groups: Vec<Group>) -> Self {
        Self::new(groups)
    }
}
