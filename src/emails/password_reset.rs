use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// The "forgot password" e-mail: Resend template `account-password-reset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordResetEmail {
    /// First name of the account, greeted by the e-mail.
    pub first_name: String,
    /// Page of the front where the user picks a new password, token included.
    pub reset_url: String,
    /// Lifetime of the link, in minutes.
    pub expire_minutes: u64,
    /// Name of the town hall shown in the header; the template falls back to "votre commune".
    pub mairie_name: Option<String>,
}

impl EmailTemplate for PasswordResetEmail {
    fn alias(&self) -> &'static str {
        "account-password-reset"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("RESET_URL".to_string(), json!(self.reset_url)),
            ("EXPIRE_MINUTES".to_string(), json!(self.expire_minutes)),
        ]);
        if let Some(mairie_name) = &self.mairie_name {
            variables.insert("MAIRIE_NAME".to_string(), json!(mairie_name));
        }
        variables
    }
}
