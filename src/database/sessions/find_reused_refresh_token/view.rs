use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

/// Session whose previous refresh token was presented again.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReusedRefreshToken {
    id: Uuid,
    user_id: i32,
    revoked: bool,
    was_active: bool,
}

impl ReusedRefreshToken {
    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }

    /// The token came back after the grace window: the session is revoked.
    #[must_use]
    pub const fn revoked(&self) -> bool {
        self.revoked
    }

    /// The session was still active (not revoked, not expired) when the token came back.
    #[must_use]
    pub const fn was_active(&self) -> bool {
        self.was_active
    }
}

/// Finds the session whose **previous** refresh token is `token_hash`, and revokes it when that
/// token was replaced at or before `grace_cutoff`.
///
/// Called only after the rotation found no session holding `token_hash`. A previous token
/// presented after the grace window means two parties hold the session's tokens (a stolen token
/// replayed, or the thief already refreshed): the whole session is revoked, the current token too.
/// Within the window it is a concurrent refresh of the same session (two fronts renewing at the
/// same time): nothing changes. The row is locked, so a rotation running at the same time waits.
///
/// `revoked_at` keeps its first value (`COALESCE`), and the logout log trigger only fires when it
/// was NULL. Answers `DbError::NotFound` when no session had this token as previous one.
#[derive(serde::Deserialize)]
pub struct FindReusedRefreshTokenQueryView {
    params: Vec<QueryParam>,
}

impl FindReusedRefreshTokenQueryView {
    #[must_use]
    pub fn new(token_hash: &str, grace_cutoff: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        Self {
            params: vec![
                QueryParam::Text(token_hash.to_string()),
                QueryParam::DateTime(grace_cutoff),
                QueryParam::DateTime(now),
            ],
        }
    }
}

impl ApiRequestDto for FindReusedRefreshTokenQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH reused AS (\
            SELECT id, user_id, rotated_at <= $2 AS revoked, \
                   (revoked_at IS NULL AND (expires_at IS NULL OR expires_at > $3)) AS was_active \
            FROM sessions WHERE previous_token_hash = $1 LIMIT 1 FOR UPDATE\
        ), revoke AS (\
            UPDATE sessions SET revoked_at = COALESCE(sessions.revoked_at, $3) \
            FROM reused WHERE sessions.id = reused.id AND reused.revoked \
            RETURNING sessions.id\
        ) SELECT row_to_json(reused) FROM reused"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for FindReusedRefreshTokenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FindReusedRefreshTokenQueryView: token_hash = [PROTECTED]"
        )
    }
}
