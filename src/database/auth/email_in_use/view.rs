use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// `1` when an account (archived or not) already holds an e-mail address, `0` otherwise.
///
/// Archived accounts count: their address stays reserved by the unique constraint of `users`.
#[derive(serde::Deserialize)]
pub struct EmailInUseQueryView {
    params: Vec<QueryParam>,
}

impl EmailInUseQueryView {
    #[must_use]
    pub fn new(email: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(email.to_string())],
        }
    }
}

impl ApiRequestDto for EmailInUseQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT CASE WHEN EXISTS (SELECT 1 FROM users WHERE lower(email) = lower($1)) THEN 1 ELSE 0 END"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for EmailInUseQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmailInUseQueryView: email = [PROTECTED]")
    }
}
