use crate::database::ids::id_to_sql_i64;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

/// Revokes every active session of `user_id` (optionally but one), and returns the ids of the
/// sessions it revoked, one JSON string per row (`fetch_all::<Uuid>`).
#[derive(serde::Deserialize)]
pub struct RevokeUserSessionsQueryView {
    user_id: u64,
    except: Option<Uuid>,
    params: Vec<QueryParam>,
}

impl RevokeUserSessionsQueryView {
    /// Revokes all the active sessions of `user_id`.
    #[must_use]
    pub fn all(user_id: u64) -> Self {
        Self {
            user_id,
            except: None,
            params: vec![QueryParam::I64(id_to_sql_i64(user_id))],
        }
    }

    /// Revokes the active sessions of `user_id` except `session_id` (the caller's).
    #[must_use]
    pub fn all_but(user_id: u64, session_id: Uuid) -> Self {
        Self {
            user_id,
            except: Some(session_id),
            params: vec![
                QueryParam::I64(id_to_sql_i64(user_id)),
                QueryParam::Uuid(session_id),
            ],
        }
    }
}

impl ApiRequestDto for RevokeUserSessionsQueryView {
    fn query_sql(&self) -> &'static str {
        if self.except.is_some() {
            "WITH revoked AS (\
                UPDATE sessions SET revoked_at = now() \
                WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL \
                AND (expires_at IS NULL OR expires_at > now()) RETURNING id\
            ) SELECT to_json(id) FROM revoked"
        } else {
            "WITH revoked AS (\
                UPDATE sessions SET revoked_at = now() \
                WHERE user_id = $1 AND revoked_at IS NULL \
                AND (expires_at IS NULL OR expires_at > now()) RETURNING id\
            ) SELECT to_json(id) FROM revoked"
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RevokeUserSessionsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeUserSessionsQueryView: user_id = {}, except = {:?}",
            self.user_id, self.except
        )
    }
}

/// The ids of the sessions of `user_id` revoked in the last `seconds`, one JSON string per row.
///
/// Read with `fetch_all::<Uuid>`. A revocation is published to Redis after its commit and that
/// write can fail: these sessions are the ones whose JWT may still be accepted, so the same
/// request replays the publication from the table (the table is the retry state, nothing is lost
/// with the process).
#[derive(serde::Deserialize)]
pub struct RecentlyRevokedSessionsQueryView {
    params: Vec<QueryParam>,
}

impl RecentlyRevokedSessionsQueryView {
    #[must_use]
    pub fn new(user_id: u64, seconds: u64) -> Self {
        Self {
            params: vec![
                QueryParam::I64(id_to_sql_i64(user_id)),
                QueryParam::I64(i64::try_from(seconds).unwrap_or(i64::MAX)),
            ],
        }
    }
}

impl ApiRequestDto for RecentlyRevokedSessionsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_json(id) FROM sessions \
         WHERE user_id = $1 AND revoked_at IS NOT NULL \
           AND revoked_at > now() - $2::bigint * INTERVAL '1 second'"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

/// Revokes the session `session_id` of `user_id` when it is still active.
///
/// Returns its id (`fetch_all::<Uuid>`: one row, or none when the session is unknown, belongs to another user,
/// or already ended). Used by the administrators (MAIR-584).
#[derive(serde::Deserialize)]
pub struct RevokeUserSessionQueryView {
    user_id: u64,
    session_id: Uuid,
    params: Vec<QueryParam>,
}

impl RevokeUserSessionQueryView {
    #[must_use]
    pub fn new(user_id: u64, session_id: Uuid) -> Self {
        Self {
            user_id,
            session_id,
            params: vec![
                QueryParam::I64(id_to_sql_i64(user_id)),
                QueryParam::Uuid(session_id),
            ],
        }
    }
}

impl ApiRequestDto for RevokeUserSessionQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH revoked AS (\
            UPDATE sessions SET revoked_at = now() \
            WHERE user_id = $1 AND id = $2 AND revoked_at IS NULL \
            AND (expires_at IS NULL OR expires_at > now()) RETURNING id\
        ) SELECT to_json(id) FROM revoked"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RevokeUserSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeUserSessionQueryView: user_id = {}, session_id = {}",
            self.user_id, self.session_id
        )
    }
}
