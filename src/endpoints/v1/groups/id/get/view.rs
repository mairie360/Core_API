use crate::database::groups::get_group::Group;
use utoipa::ToSchema;

/// Detail of a group.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupResultView {
    /// The requested group.
    group: Group,
}

impl GetGroupResultView {
    pub const fn new(group: Group) -> Self {
        Self { group }
    }
}
