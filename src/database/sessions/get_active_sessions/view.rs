use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetActiveSessionsQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetActiveSessionsQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I64(user_id as i64)],
        }
    }

    #[must_use]
    pub const fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

// No `cache_key` (MAIR-267): the list must reflect logins and revocations at once, but no write
// path invalidated the cached list and it had no TTL, so a Redis without ACL (local stacks)
// served a stale list forever. Under the chart's ACL its unprefixed key was refused anyway.
impl ApiRequestDto for GetActiveSessionsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT * FROM v_sessions WHERE user_id = $1 AND is_active = true) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveSessionsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetActiveSessionsQueryView: user_id = {}", self.user_id)
    }
}
