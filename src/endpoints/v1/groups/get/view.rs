use crate::database::groups::get_group::Group;
use utoipa::ToSchema;

/// List of groups.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupsResultView {
    /// Groups the signed-in user is a member of. Empty if they belong to none.
    groups: Vec<Group>,
}

impl GetGroupsResultView {
    pub const fn new(groups: Vec<Group>) -> Self {
        Self { groups }
    }
}

impl From<Vec<Group>> for GetGroupsResultView {
    fn from(groups: Vec<Group>) -> Self {
        Self::new(groups)
    }
}
