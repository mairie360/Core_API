use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// The non-archived account registered under an e-mail address, as the password reset e-mail
/// needs it: no row when there is none (an archived account gets no reset e-mail, MAIR-390).
#[derive(serde::Deserialize)]
pub struct ResetRecipientQueryView {
    params: Vec<QueryParam>,
}

impl ResetRecipientQueryView {
    #[must_use]
    pub fn new(email: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(email.to_string())],
        }
    }
}

impl ApiRequestDto for ResetRecipientQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT first_name FROM users \
            WHERE email = $1 AND NOT COALESCE(is_archived, FALSE)\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ResetRecipientQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResetRecipientQueryView: email = [PROTECTED]")
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ResetRecipientQueryResultView {
    first_name: String,
}

impl ResetRecipientQueryResultView {
    #[must_use]
    pub fn first_name(&self) -> &str {
        &self.first_name
    }
}
