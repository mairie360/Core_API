use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// The invitation e-mail of an account created by an administrator: Resend template
/// `account-invitation`. Its link opens the password page of the login front.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInvitationEmail {
    /// First name of the account, greeted by the e-mail.
    pub first_name: String,
    /// Page of the front where the user sets their password, token included.
    pub set_password_url: String,
    /// Lifetime of the link, in days.
    pub expire_days: u64,
    /// Name of the town hall shown in the header; the template falls back to "votre commune".
    pub mairie_name: Option<String>,
}

impl EmailTemplate for AccountInvitationEmail {
    fn alias(&self) -> &'static str {
        "account-invitation"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("SET_PASSWORD_URL".to_string(), json!(self.set_password_url)),
            ("EXPIRE_DAYS".to_string(), json!(self.expire_days)),
        ]);
        if let Some(mairie_name) = &self.mairie_name {
            variables.insert("MAIRIE_NAME".to_string(), json!(mairie_name));
        }
        variables
    }
}
