use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct UnsetFirstConnectionQueryView {
    user_id: u64,
    password: String,
    params: Vec<QueryParam>,
}

impl UnsetFirstConnectionQueryView {
    #[must_use]
    pub fn new(user_id: u64, password: &str) -> Self {
        Self {
            user_id,
            password: password.to_string(),
            params: vec![
                QueryParam::Text(password.to_string()),
                QueryParam::I32(user_id as i32),
            ],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }
}

impl ApiRequestDto for UnsetFirstConnectionQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET first_connect = false AND password = $1 WHERE id = $2"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for UnsetFirstConnectionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UnsetFirstConnectionQueryView: user_id = {}, password = [PROTECTED]",
            self.user_id
        )
    }
}
