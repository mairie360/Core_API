use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Replaces the refresh token of the active session holding `current_hash` by `new_hash`.
///
/// Returns that session as an `ActiveSession` (`id`, `user_id`). Answers `DbError::NotFound` when
/// no active session holds `current_hash` (unknown, revoked, expired or already rotated token).
///
/// The outer `token_hash = $1` is re-checked on the locked row: of two concurrent refreshes with
/// the same token, only one rotates it, the other finds nothing (MAIR-390).
#[derive(serde::Deserialize)]
pub struct RotateRefreshTokenQueryView {
    params: Vec<QueryParam>,
}

impl RotateRefreshTokenQueryView {
    #[must_use]
    pub fn new(current_hash: &str, new_hash: &str) -> Self {
        Self {
            params: vec![
                QueryParam::Text(current_hash.to_string()),
                QueryParam::Text(new_hash.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for RotateRefreshTokenQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH rotated AS (\
            UPDATE sessions SET token_hash = $2 \
            WHERE token_hash = $1 \
              AND id = (SELECT id FROM v_sessions WHERE token_hash = $1 AND is_active = true LIMIT 1) \
            RETURNING id, user_id\
        ) SELECT row_to_json(rotated) FROM rotated"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RotateRefreshTokenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RotateRefreshTokenQueryView: current_hash = [PROTECTED], new_hash = [PROTECTED]"
        )
    }
}
