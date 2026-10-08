use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Rewrites a credential after a successful authentication (signature counter, backup flags)
/// and stamps `last_used_at`.
#[derive(Debug, serde::Deserialize)]
pub struct UpdatePasskeyCredentialQueryView {
    passkey_id: i32,
    params: Vec<QueryParam>,
}

impl UpdatePasskeyCredentialQueryView {
    #[must_use]
    pub fn new(passkey_id: i32, passkey: &str) -> Self {
        Self {
            passkey_id,
            params: vec![
                QueryParam::I32(passkey_id),
                QueryParam::Text(passkey.to_string()),
            ],
        }
    }

    #[must_use]
    pub const fn passkey_id(&self) -> i32 {
        self.passkey_id
    }
}

impl ApiRequestDto for UpdatePasskeyCredentialQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE user_passkeys SET passkey = $2::jsonb, last_used_at = now() WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for UpdatePasskeyCredentialQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UpdatePasskeyCredentialQueryView: passkey_id = {}",
            self.passkey_id
        )
    }
}
