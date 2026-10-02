use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Id of the non-archived account registered under an e-mail address, `0` when there is none.
///
/// Used by the password reset flow: an archived account must neither receive a reset e-mail nor
/// get its password changed (MAIR-390).
#[derive(serde::Deserialize)]
pub struct GetActiveUserIdByEmailQueryView {
    params: Vec<QueryParam>,
}

impl GetActiveUserIdByEmailQueryView {
    #[must_use]
    pub fn new(email: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(email.to_string())],
        }
    }
}

impl ApiRequestDto for GetActiveUserIdByEmailQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT COALESCE((\
            SELECT id FROM users WHERE email = $1 AND NOT COALESCE(is_archived, FALSE)\
        ), 0)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveUserIdByEmailQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetActiveUserIdByEmailQueryView: email = [PROTECTED]")
    }
}
