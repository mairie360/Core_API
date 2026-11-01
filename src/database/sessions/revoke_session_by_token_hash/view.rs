use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Revokes the active session whose refresh token digest is `token_hash`, whoever owns it, and
/// returns its id as one JSON string (`fetch_all::<Uuid>`), also when that session was already
/// revoked: a logout retried after a failed Redis publication publishes it again. Empty when no
/// session has this token.
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
        // The second SELECT reads the snapshot taken before the UPDATE: it only adds a session
        // that was revoked before this call.
        "WITH revoked AS (\
            UPDATE sessions SET revoked_at = $1 \
            WHERE token_hash = $2 AND revoked_at IS NULL RETURNING id\
        ) SELECT to_json(id) FROM (\
            SELECT id FROM revoked \
            UNION SELECT id FROM sessions WHERE token_hash = $2 AND revoked_at IS NOT NULL\
        ) AS ids"
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
