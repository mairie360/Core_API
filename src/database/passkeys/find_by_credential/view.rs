use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The passkey registered under a credential id (hex), with its account: how a discoverable
/// sign-in (no e-mail typed) finds who is signing in. `NotFound` when unknown.
#[derive(Debug, serde::Deserialize)]
pub struct FindPasskeyByCredentialQueryView {
    params: Vec<QueryParam>,
}

impl FindPasskeyByCredentialQueryView {
    #[must_use]
    pub fn new(credential_id: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(credential_id.to_string())],
        }
    }
}

impl ApiRequestDto for FindPasskeyByCredentialQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT p.id, p.user_id, p.passkey, COALESCE(u.is_archived, false) AS is_archived \
            FROM user_passkeys p JOIN users u ON u.id = p.user_id \
            WHERE p.credential_id = decode($1, 'hex')\
         ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for FindPasskeyByCredentialQueryView {
    // The credential id identifies a device: kept out of the logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FindPasskeyByCredentialQueryView")
    }
}

/// Result of [`FindPasskeyByCredentialQueryView`].
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredPasskey {
    pub id: i32,
    pub user_id: i32,
    /// The credential as serialised by webauthn-rs.
    pub passkey: serde_json::Value,
    pub is_archived: bool,
}
