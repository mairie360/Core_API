use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

/// Whether the session `id` of `user_id` is still active (not revoked, not expired).
#[derive(serde::Deserialize)]
pub struct IsSessionActiveQueryView {
    id: Uuid,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl IsSessionActiveQueryView {
    #[must_use]
    pub fn new(id: Uuid, user_id: u64) -> Self {
        Self {
            id,
            user_id,
            params: vec![QueryParam::Uuid(id), QueryParam::I64(user_id as i64)],
        }
    }
}

impl ApiRequestDto for IsSessionActiveQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM v_sessions WHERE id = $1 AND user_id = $2 AND is_active = true)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsSessionActiveQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsSessionActiveQueryView: id = {}, user_id = {}",
            self.id, self.user_id
        )
    }
}
