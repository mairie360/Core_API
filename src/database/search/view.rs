use crate::database::ids::id_to_sql_i64;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// What a global search result is. Core finds agents only.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    /// A non-archived user of the directory; `id` is reusable in `GET /api/v1/user/{id}/`.
    Agent,
}

/// One result of the global search.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct SearchItem {
    pub kind: SearchKind,
    /// Id of the agent.
    #[schema(example = 12)]
    pub id: i64,
    /// Full name, « first name last name ».
    #[schema(example = "Jean Dupont")]
    pub title: String,
    /// The agent's e-mail address.
    #[schema(example = "jean.dupont@mairie360.fr")]
    pub excerpt: Option<String>,
    /// Always `null` for an agent (no parent item).
    #[schema(example = json!(null))]
    pub parent_id: Option<i64>,
    /// Last change of the account (UTC, RFC 3339).
    #[schema(format = DateTime, example = "2026-11-20T09:30:00Z")]
    pub updated_at: String,
}

/// Agents of the directory matching a global search (`GET /api/v1/search`): the non-archived
/// users whose names (both orders) or e-mail contain the term, matched on the expression of the
/// trigram index `idx_users_search_text_trgm`. Names starting with the term come first, then the
/// most recently updated accounts.
#[derive(Debug, serde::Deserialize)]
pub struct GlobalSearchQueryView {
    params: Vec<QueryParam>,
}

impl GlobalSearchQueryView {
    /// `pattern` is the term already escaped for `LIKE` (see `like_escape`), `limit` bounded by
    /// the endpoint.
    pub fn new(pattern: &str, limit: u64) -> Self {
        Self {
            params: vec![
                QueryParam::Text(pattern.to_string()),
                QueryParam::I64(id_to_sql_i64(limit)),
            ],
        }
    }
}

impl ApiRequestDto for GlobalSearchQueryView {
    fn query_sql(&self) -> &'static str {
        concat!(
            "SELECT jsonb_build_object( \
                'kind', 'agent', \
                'id', u.id, \
                'title', u.first_name || ' ' || u.last_name, \
                'excerpt', u.email, \
                'parent_id', NULL, \
                'updated_at', to_char(COALESCE(u.updated_at, u.created_at) AT TIME ZONE 'UTC', \
                                      'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')) \
             FROM users u \
             WHERE COALESCE(u.is_archived, false) = false \
               AND ",
            crate::user_search_text_sql!(),
            " LIKE '%' || lower($1) || '%' \
             ORDER BY (lower(u.first_name || ' ' || u.last_name) LIKE lower($1) || '%' \
                       OR lower(u.last_name) LIKE lower($1) || '%') DESC, \
                      COALESCE(u.updated_at, u.created_at) DESC NULLS LAST, u.id DESC \
             LIMIT $2"
        )
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GlobalSearchQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlobalSearchQueryView: {:?}", self.params)
    }
}
