use crate::database::ids::id_to_sql_i64;
use crate::endpoints::pagination::{PageQuery, DEFAULT_LIMIT};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Sessions of `user_id`, active and ended, newest first. `new` returns the latest
/// [`DEFAULT_LIMIT`]; `page` any page (MAIR-425: the history grows with every login).
#[derive(serde::Deserialize)]
pub struct GetSessionsByUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetSessionsByUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![
                QueryParam::I64(id_to_sql_i64(user_id)),
                QueryParam::I64(id_to_sql_i64(DEFAULT_LIMIT)),
                QueryParam::I64(0),
            ],
        }
    }

    /// Page `limit` / `offset` of the same list (MAIR-425).
    #[must_use]
    pub fn page(user_id: u64, page: PageQuery) -> Self {
        let mut view = Self::new(user_id);
        view.params.truncate(1);
        view.params
            .push(QueryParam::I64(id_to_sql_i64(page.limit())));
        view.params
            .push(QueryParam::I64(id_to_sql_i64(page.offset())));
        view
    }

    #[must_use]
    pub const fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

// No `cache_key` (MAIR-267): the list must reflect logins and revocations at once, but no write
// path invalidated the cached list and it had no TTL, so a Redis without ACL (local stacks)
// served a stale list forever. Under the chart's ACL its unprefixed key was refused anyway.
impl ApiRequestDto for GetSessionsByUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT * FROM sessions WHERE user_id = $1 \
            ORDER BY created_at DESC, id LIMIT $2 OFFSET $3\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetSessionsByUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetSessionsByUserQueryView: user_id = {}", self.user_id)
    }
}
