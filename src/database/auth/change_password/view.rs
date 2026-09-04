use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct ChangePasswordQueryView {
    password: String,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl ChangePasswordQueryView {
    pub fn new(password: &str, user_id: u64) -> Self {
        Self {
            password: password.to_string(),
            user_id,
            params: vec![
                QueryParam::Text(password.to_string()),
                QueryParam::I32(user_id as i32),
            ],
        }
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for ChangePasswordQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET password = $1 WHERE id = $2"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ChangePasswordQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChangePasswordQueryView: password = [PROTECTED]")
    }
}
