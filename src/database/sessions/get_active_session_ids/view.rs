use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Ids of the active sessions of `user_id`, one JSON string per row (`fetch_all::<Uuid>`).
#[derive(serde::Deserialize)]
pub struct GetActiveSessionIdsQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetActiveSessionIdsQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I64(user_id as i64)],
        }
    }
}

impl ApiRequestDto for GetActiveSessionIdsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_json(id) FROM v_sessions WHERE user_id = $1 AND is_active = true"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveSessionIdsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetActiveSessionIdsQueryView: user_id = {}",
            self.user_id
        )
    }
}
