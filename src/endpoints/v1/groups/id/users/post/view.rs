use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostUserGroupView {
    user_id: u64,
    group_id: u64,
}

impl PostUserGroupView {
    pub fn new(user_id: u64, group_id: u64) -> Self {
        Self { user_id, group_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}
