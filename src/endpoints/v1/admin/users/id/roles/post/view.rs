use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct AddRoleToUserView {
    role_id: u64,
    user_id: u64,
}

impl AddRoleToUserView {
    pub fn role_id(&self) -> u64 {
        self.role_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AddRoleToUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddRoleToUserView {{ role_id: {}, user_id: {} }}",
            self.role_id, self.user_id
        )
    }
}
