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
