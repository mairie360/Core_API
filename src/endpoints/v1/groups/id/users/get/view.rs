use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupUsersResultView {
    users: Vec<u64>,
}

impl GetGroupUsersResultView {
    pub fn new(users: Vec<u64>) -> Self {
        Self { users }
    }
}

impl From<Vec<i32>> for GetGroupUsersResultView {
    fn from(users: Vec<i32>) -> Self {
        Self::new(users.into_iter().map(|u| u as u64).collect())
    }
}
