use crate::database::ids::{id_to_sql, id_to_sql_i64};
use crate::endpoints::pagination::{PageQuery, DEFAULT_LIMIT};
use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

#[derive(serde::Deserialize)]
pub struct GetUserGroupsQuerView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetUserGroupsQuerView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
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
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for GetUserGroupsQuerView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT * FROM groups \
            WHERE id IN (SELECT group_id FROM group_members WHERE user_id = $1) \
            ORDER BY name, id LIMIT $2 OFFSET $3\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserGroupsQuerView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroups: user_id = {}", self.user_id)
    }
}
