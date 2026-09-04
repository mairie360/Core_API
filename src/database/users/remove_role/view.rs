use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct RemoveRolesQueryView {
    role_id: u64,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl RemoveRolesQueryView {
    pub fn new(role_id: u64, user_id: u64) -> Self {
        Self {
            role_id,
            user_id,
            params: vec![
                QueryParam::I32(role_id as i32),
                QueryParam::I32(user_id as i32),
            ],
        }
    }

    pub fn role_id(&self) -> u64 {
        self.role_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for RemoveRolesQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM user_roles WHERE role_id = $1 AND user_id = $2"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RemoveRolesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RemoveRolesQueryView: role_id = {}, user_id = {}",
            self.role_id, self.user_id
        )
    }
}
