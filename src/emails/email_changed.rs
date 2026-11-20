use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// The security e-mail sent to the OLD address once the change was confirmed: Resend template
/// `account-email-changed`. It lets the former owner of the mailbox notice an unwanted change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailChangedEmail {
    /// First name of the account, greeted by the e-mail.
    pub first_name: String,
    /// The address the account now uses.
    pub new_email: String,
    /// When the change was confirmed, RFC 3339 UTC.
    pub date: String,
    /// Name of the town hall shown in the header; the template falls back to "votre commune".
    pub mairie_name: Option<String>,
}

impl EmailTemplate for EmailChangedEmail {
    fn alias(&self) -> &'static str {
        "account-email-changed"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("NEW_EMAIL".to_string(), json!(self.new_email)),
            ("DATE".to_string(), json!(self.date)),
        ]);
        if let Some(mairie_name) = &self.mairie_name {
            variables.insert("MAIRIE_NAME".to_string(), json!(mairie_name));
        }
        variables
    }
}
