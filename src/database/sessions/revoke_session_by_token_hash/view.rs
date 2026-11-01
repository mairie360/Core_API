use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Revokes the active session whose refresh token digest is `token_hash`, whoever owns it, and
/// returns its id as one JSON string (`fetch_all::<Uuid>`, empty when nothing was revoked).
///
/// Holding the refresh token is the proof (MAIR-692): it already renews the session without a JWT
/// (`POST /api/v1/sessions/refresh`), so it may also end it.
#[derive(serde::Deserialize)]
pub struct RevokeSessionByTokenHashQueryView {
    params: Vec<QueryParam>,
}

impl RevokeSessionByTokenHashQueryView {
    #[must_use]
    pub fn new(token_hash: &str) -> Self {
        Self {
            params: vec![
                QueryParam::DateTime(chrono::Utc::now()),
                QueryParam::Text(token_hash.to_string()),
            ],
        }
    }
}

impl ApiRequestDto for RevokeSessionByTokenHashQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH revoked AS (\
            UPDATE sessions SET revoked_at = $1 \
            WHERE token_hash = $2 AND revoked_at IS NULL RETURNING id\
        ) SELECT to_json(id) FROM revoked"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RevokeSessionByTokenHashQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeSessionByTokenHashQueryView: token_hash = [PROTECTED]"
        )
    }
}
