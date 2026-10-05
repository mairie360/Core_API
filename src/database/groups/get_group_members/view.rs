use crate::database::ids::{id_to_sql, id_to_sql_i64};
use crate::endpoints::pagination::{PageQuery, DEFAULT_LIMIT};
use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

#[derive(serde::Deserialize)]
pub struct GetGroupUsersQueryView {
    group_id: u64,
    params: Vec<QueryParam>,
}

impl GetGroupUsersQueryView {
    #[must_use]
    pub fn new(group_id: u64) -> Self {
        Self {
            group_id,
            params: vec![
                QueryParam::I32(id_to_sql(group_id)),
                QueryParam::I64(id_to_sql_i64(DEFAULT_LIMIT)),
                QueryParam::I64(0),
            ],
        }
    }

    /// Page `limit` / `offset` of the same list (MAIR-425).
    #[must_use]
    pub fn page(group_id: u64, page: PageQuery) -> Self {
        let mut view = Self::new(group_id);
        view.params.truncate(1);
        view.params
            .push(QueryParam::I64(id_to_sql_i64(page.limit())));
        view.params
            .push(QueryParam::I64(id_to_sql_i64(page.offset())));
        view
    }

    #[must_use]
    pub const fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl ApiRequestDto for GetGroupUsersQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(user_id) FROM group_members WHERE group_id = $1 \
         ORDER BY user_id LIMIT $2 OFFSET $3"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetGroupUsersQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroupUsersQueryView: group_id = {}", self.group_id)
    }
}
