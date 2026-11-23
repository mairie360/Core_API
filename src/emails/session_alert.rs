use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// French wording of a suspicion level, written into the templates (a Resend template cannot branch).
#[must_use]
pub fn certainty_label(level: &str) -> &'static str {
    if level == "certain" {
        "certaine"
    } else {
        "possible"
    }
}

/// The security e-mail sent to an agent whose session was used from another device (MAIR-773).
///
/// Resend template `account-session-alert`: the session is already closed; the e-mail tells the agent
/// when and from where it was used and asks them to change their password if it was not them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAlertEmail {
    pub first_name: String,
    /// `certaine` or `possible` ([`certainty_label`]).
    pub certainty: String,
    pub device: String,
    pub ip_address: String,
    /// ISO 8601 date of the use (UTC).
    pub date: String,
    pub mairie_name: Option<String>,
}

impl EmailTemplate for SessionAlertEmail {
    fn alias(&self) -> &'static str {
        "account-session-alert"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("CERTAINTY".to_string(), json!(self.certainty)),
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

/// The same alert, sent to every active administrator of the instance (MAIR-773).
///
/// Resend template `admin-session-alert`: names the agent so an administrator can follow up from the
/// Security tab of the administration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminSessionAlertEmail {
    /// First and last name of the agent.
    pub agent_name: String,
    pub certainty: String,
    pub device: String,
    pub ip_address: String,
    pub date: String,
    pub mairie_name: Option<String>,
}

impl EmailTemplate for AdminSessionAlertEmail {
    fn alias(&self) -> &'static str {
        "admin-session-alert"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("AGENT_NAME".to_string(), json!(self.agent_name)),
            ("CERTAINTY".to_string(), json!(self.certainty)),
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
    fn mair_773_session_alerts_fill_their_template_variables() {
        let agent = SessionAlertEmail {
            first_name: "Alice".into(),
            certainty: certainty_label("certain").into(),
            device: "Firefox".into(),
            ip_address: "203.0.113.7".into(),
            date: "2026-11-23T08:12:00Z".into(),
            mairie_name: None,
        };
        assert_eq!(agent.alias(), "account-session-alert");
        let variables = agent.variables();
        assert_eq!(variables["CERTAINTY"], json!("certaine"));
        assert_eq!(variables["USER_FIRST_NAME"], json!("Alice"));
        assert!(!variables.contains_key("MAIRIE_NAME"));

        let admin = AdminSessionAlertEmail {
            agent_name: "Alice Martin".into(),
            certainty: certainty_label("possible").into(),
            device: "Firefox".into(),
            ip_address: "203.0.113.7".into(),
            date: "2026-11-23T08:12:00Z".into(),
            mairie_name: Some("Mairie de Test".into()),
        };
        assert_eq!(admin.alias(), "admin-session-alert");
        let variables = admin.variables();
        assert_eq!(variables["AGENT_NAME"], json!("Alice Martin"));
        assert_eq!(variables["CERTAINTY"], json!("possible"));
        assert_eq!(variables["MAIRIE_NAME"], json!("Mairie de Test"));
    }
}
