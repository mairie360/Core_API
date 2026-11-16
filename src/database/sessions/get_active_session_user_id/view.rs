use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Returns the `user_id` of the active session tied to a refresh token.
///
/// Used by the JWT refresh, which must work when the JWT has expired: the refresh token is then
/// the only proof of identity. No row (unknown, revoked or expired token) gives
/// `DbError::NotFound`.
#[derive(serde::Deserialize)]
pub struct GetActiveSessionUserIdQueryView {
    token_hash: String,
    params: Vec<QueryParam>,
}

impl GetActiveSessionUserIdQueryView {
    #[must_use]
    pub fn new(token_hash: &str) -> Self {
        Self {
            token_hash: token_hash.to_string(),
            params: vec![QueryParam::Text(token_hash.to_string())],
        }
    }

    #[must_use]
    pub fn get_token_hash(&self) -> &str {
        &self.token_hash
    }
}

impl ApiRequestDto for GetActiveSessionUserIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT user_id FROM v_sessions WHERE token_hash = $1 AND is_active = true LIMIT 1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveSessionUserIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetActiveSessionUserIdQueryView: token_hash = [PROTECTED]"
        )
    }
}
