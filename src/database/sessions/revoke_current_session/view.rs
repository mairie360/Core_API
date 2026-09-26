use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

/// Revokes the session `id` of `user_id`. Idempotent: an already revoked session keeps its
/// original `revoked_at`.
#[derive(serde::Deserialize)]
pub struct RevokeCurrentSessionQueryView {
    id: Uuid,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl RevokeCurrentSessionQueryView {
    #[must_use]
    pub fn new(id: Uuid, user_id: u64) -> Self {
        Self {
            id,
            user_id,
            params: vec![QueryParam::Uuid(id), QueryParam::I64(user_id as i64)],
        }
    }
}

impl ApiRequestDto for RevokeCurrentSessionQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE sessions SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RevokeCurrentSessionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeCurrentSessionQueryView: id = {}, user_id = {}",
            self.id, self.user_id
        )
    }
}
