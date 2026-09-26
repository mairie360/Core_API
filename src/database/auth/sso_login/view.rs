use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Finds the account matching an e-mail verified by an external identity provider (Keycloak).
///
/// The match ignores case, since Keycloak lowercases e-mails while accounts created in Core
/// keep the case they were typed with. An exact match wins if several accounts only differ by
/// case (the `users.email` constraint is case-sensitive).
#[derive(serde::Deserialize)]
pub struct SsoLoginUserQueryView {
    email: String,
    params: Vec<QueryParam>,
}

impl SsoLoginUserQueryView {
    #[must_use]
    pub fn new(email: &str) -> Self {
        Self {
            email: email.to_string(),
            params: vec![QueryParam::Text(email.to_string())],
        }
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }
}

impl ApiRequestDto for SsoLoginUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT id, COALESCE(is_archived, false) AS is_archived \
            FROM users \
            WHERE lower(email) = lower($1) \
            ORDER BY (email = $1) DESC, id \
            LIMIT 1\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for SsoLoginUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SsoLoginUserQueryView: email = {}", self.email)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SsoLoginUserQueryResultView {
    #[serde(rename = "id")]
    user_id: i32,
    is_archived: bool,
}

impl SsoLoginUserQueryResultView {
    #[must_use]
    pub const fn new(user_id: i32, is_archived: bool) -> Self {
        Self {
            user_id,
            is_archived,
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }

    #[must_use]
    pub const fn is_archived(&self) -> bool {
        self.is_archived
    }
}
