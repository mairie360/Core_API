use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostUserGroupView {
    user_id: u64,
    group_id: u64,
}

impl PostUserGroupView {
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    pub const fn group_id(&self) -> u64 {
        self.group_id
    }
}
