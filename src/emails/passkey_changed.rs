use std::collections::HashMap;

use mairie360_api_lib::email::EmailTemplate;
use serde_json::{json, Value};

/// What happened to the passkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasskeyChange {
    Added,
    Removed,
}

impl PasskeyChange {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
        }
    }
}

/// The security e-mail sent when a passkey is added or removed (MAIR-810).
///
/// Resend template `account-passkey-changed`. It tells the agent what changed, from which device,
/// when and from which address, so that an unexpected change is noticed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasskeyChangedEmail {
    /// First name of the account, greeted by the e-mail.
    pub first_name: String,
    pub change: PasskeyChange,
    /// Name the agent gave to the passkey.
    pub label: String,
    /// User agent of the request (`device_info`), empty when unknown.
    pub device: String,
    /// ISO 8601 date of the change (UTC).
    pub date: String,
    /// Address the request came from.
    pub ip_address: String,
    /// Name of the town hall shown in the header; the template falls back to "votre commune".
    pub mairie_name: Option<String>,
}

impl EmailTemplate for PasskeyChangedEmail {
    fn alias(&self) -> &'static str {
        "account-passkey-changed"
    }

    fn variables(&self) -> HashMap<String, Value> {
        let mut variables = HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("CHANGE".to_string(), json!(self.change.as_str())),
            ("PASSKEY_LABEL".to_string(), json!(self.label)),
            ("DEVICE".to_string(), json!(self.device)),
            ("DATE".to_string(), json!(self.date)),
            ("IP_ADDRESS".to_string(), json!(self.ip_address)),
        ]);
        if let Some(mairie_name) = &self.mairie_name {
            variables.insert("MAIRIE_NAME".to_string(), json!(mairie_name));
        }
        variables
    }
}
