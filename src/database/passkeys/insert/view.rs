use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Registers a passkey for a user and returns its [`super::super::PasskeySummary`].
///
/// `credential_id` is hex-encoded ([`super::super::credential_id_hex`]), `passkey` the JSON
/// serialisation of the credential. A credential id already registered (any account) raises a
/// unique violation.
#[derive(Debug, serde::Deserialize)]
pub struct InsertPasskeyQueryView {
    user_id: u64,
    label: String,
    params: Vec<QueryParam>,
}

impl InsertPasskeyQueryView {
    #[must_use]
    pub fn new(user_id: u64, credential_id: &str, passkey: &str, label: &str) -> Self {
        Self {
            user_id,
            label: label.to_string(),
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::Text(credential_id.to_string()),
                QueryParam::Text(passkey.to_string()),
                QueryParam::Text(label.to_string()),
            ],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl ApiRequestDto for InsertPasskeyQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH inserted AS (\
            INSERT INTO user_passkeys (user_id, credential_id, passkey, label) \
            VALUES ($1, decode($2, 'hex'), $3::jsonb, $4) \
            RETURNING id, label, created_at, last_used_at\
         ) SELECT row_to_json(inserted) FROM inserted"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for InsertPasskeyQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InsertPasskeyQueryView: user_id = {}", self.user_id)
    }
}
