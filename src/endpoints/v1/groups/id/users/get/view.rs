use crate::database::ids::id_from_sql;
use utoipa::ToSchema;

/// Members of a group.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupUsersResultView {
    /// Ids of the group members, to pass to `GET /api/v1/user/?ids=…` to get
    /// their records. Empty if the group has no member.
    #[schema(example = json!([1, 2, 5]))]
    users: Vec<u64>,
}

impl GetGroupUsersResultView {
    pub const fn new(users: Vec<u64>) -> Self {
        Self { users }
    }
}

impl From<Vec<i32>> for GetGroupUsersResultView {
    fn from(users: Vec<i32>) -> Self {
        Self::new(users.into_iter().map(id_from_sql).collect())
    }
}
