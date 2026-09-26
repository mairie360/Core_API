use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

/// Session id and owner of an active session.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveSession {
    id: Uuid,
    user_id: i32,
}

impl ActiveSession {
    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }
}

/// Active (not revoked, not expired) session holding this refresh token. Answers
/// `DbError::NotFound` otherwise.
#[derive(serde::Deserialize)]
pub struct GetActiveSessionByTokenQueryView {
    params: Vec<QueryParam>,
}

impl GetActiveSessionByTokenQueryView {
    #[must_use]
    pub fn new(token_hash: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(token_hash.to_string())],
        }
    }
}

impl ApiRequestDto for GetActiveSessionByTokenQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT id, user_id FROM v_sessions WHERE token_hash = $1 AND is_active = true LIMIT 1\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveSessionByTokenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetActiveSessionByTokenQueryView: token_hash = [PROTECTED]"
        )
    }
}
