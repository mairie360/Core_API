use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// The e-mail sent to the NEW address of an agent who asked to change it: Resend template
/// `account-email-change-confirm`. The address only becomes the account's once its link is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailChangeConfirmationEmail {
    /// First name of the account, greeted by the e-mail.
    pub first_name: String,
    /// Page of the login front that confirms the change, token included.
    pub confirm_url: String,
    /// The address being confirmed.
    pub new_email: String,
    /// Lifetime of the link, in minutes.
    pub expire_minutes: u64,
    /// Name of the town hall shown in the header; the template falls back to "votre commune".
    pub mairie_name: Option<String>,
}

impl EmailTemplate for EmailChangeConfirmationEmail {
    fn alias(&self) -> &'static str {
        "account-email-change-confirm"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("CONFIRM_URL".to_string(), json!(self.confirm_url)),
            ("NEW_EMAIL".to_string(), json!(self.new_email)),
            ("EXPIRE_MINUTES".to_string(), json!(self.expire_minutes)),
        ]);
        if let Some(mairie_name) = &self.mairie_name {
            variables.insert("MAIRIE_NAME".to_string(), json!(mairie_name));
        }
        variables
    }
}
