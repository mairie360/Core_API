use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// The passkeys of a user ([`super::super::PasskeySummary`] rows), oldest first.
#[derive(Debug, serde::Deserialize)]
pub struct ListPasskeysQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl ListPasskeysQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for ListPasskeysQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT id, label, created_at, last_used_at \
            FROM user_passkeys WHERE user_id = $1 \
            ORDER BY created_at, id\
         ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ListPasskeysQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListPasskeysQueryView: user_id = {}", self.user_id)
    }
}
