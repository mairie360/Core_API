use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// A non-archived account that has no password yet (an account created by the former Keycloak
/// synchronisation, which never set one).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserWithoutPassword {
    pub id: i64,
    pub first_name: String,
    pub email: String,
}

/// The non-archived accounts without a password, by id (`fetch_all::<UserWithoutPassword>`).
///
/// An account that signs in with a passkey only is listed too: it has no password either, and
/// the invitation is the way to set one.
#[derive(Debug, Default, Deserialize)]
pub struct ListUsersWithoutPasswordQueryView {
    params: Vec<QueryParam>,
}

impl ListUsersWithoutPasswordQueryView {
    #[must_use]
    pub const fn new() -> Self {
        Self { params: Vec::new() }
    }
}

impl ApiRequestDto for ListUsersWithoutPasswordQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT id::bigint AS id, first_name, email FROM users \
         WHERE password IS NULL AND is_archived = FALSE ORDER BY id) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ListUsersWithoutPasswordQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListUsersWithoutPasswordQueryView")
    }
}
