use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Stored password (argon2id hash, or a legacy plaintext value) of a non-archived account.
/// Answers `DbError::NotFound` for an unknown or archived account.
#[derive(serde::Deserialize)]
pub struct GetUserPasswordQueryView {
    params: Vec<QueryParam>,
}

impl GetUserPasswordQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for GetUserPasswordQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT password FROM users WHERE id = $1 AND NOT COALESCE(is_archived, FALSE)\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserPasswordQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserPasswordQueryView: user_id = {:?}", self.params)
    }
}

/// Row of [`GetUserPasswordQueryView`].
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct UserPassword {
    /// `None` for an account that only signs in through Keycloak.
    password: Option<String>,
}

impl UserPassword {
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}
