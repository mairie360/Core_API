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
            params: vec![QueryParam::I64(user_id as i64)],
        }
    }

    /// Revokes the active sessions of `user_id` except `session_id` (the caller's).
    #[must_use]
    pub fn all_but(user_id: u64, session_id: Uuid) -> Self {
        Self {
            user_id,
            except: Some(session_id),
            params: vec![
                QueryParam::I64(user_id as i64),
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
