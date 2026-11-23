use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// The security e-mail sent at every new sign-in of an account that turned the alerts on
/// (MAIR-823, `user_notification_settings.login_alerts`).
///
/// Resend template `account-login-alert`: it tells the agent from which device, when and from which
/// address their account was just opened, so that an unexpected sign-in is noticed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAlertEmail {
    /// First name of the account, greeted by the e-mail.
    pub first_name: String,
    /// User agent of the sign-in (`device_info`), empty when unknown.
    pub device: String,
    /// Address the sign-in came from.
    pub ip_address: String,
    /// ISO 8601 date of the sign-in (UTC).
    pub date: String,
    /// Name of the town hall shown in the header; the template falls back to "votre commune".
    pub mairie_name: Option<String>,
}

impl EmailTemplate for LoginAlertEmail {
    fn alias(&self) -> &'static str {
        "account-login-alert"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("DEVICE".to_string(), json!(self.device)),
            ("IP_ADDRESS".to_string(), json!(self.ip_address)),
            ("DATE".to_string(), json!(self.date)),
        ]);
        if let Some(mairie_name) = &self.mairie_name {
            variables.insert("MAIRIE_NAME".to_string(), json!(mairie_name));
        }
        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mair_823_login_alert_fills_the_template_variables() {
        let email = LoginAlertEmail {
            first_name: "Alice".into(),
            device: "Firefox".into(),
            ip_address: "203.0.113.7".into(),
            date: "2026-11-21T09:30:00Z".into(),
            mairie_name: Some("Mairie de Test".into()),
        };
        assert_eq!(email.alias(), "account-login-alert");
        let variables = email.variables();
        assert_eq!(variables["USER_FIRST_NAME"], json!("Alice"));
        assert_eq!(variables["DEVICE"], json!("Firefox"));
        assert_eq!(variables["IP_ADDRESS"], json!("203.0.113.7"));
        assert_eq!(variables["DATE"], json!("2026-11-21T09:30:00Z"));
        assert_eq!(variables["MAIRIE_NAME"], json!("Mairie de Test"));
    }
}
