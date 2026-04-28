use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::database::sessions::Session;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AuditSessionSchema {
    id: String,
    user_id: i32,
    device_info: String,
    ip_address: String,
    created_at: String,
    expires_at: String,
    revoked_at: Option<String>,
}

impl From<Session> for AuditSessionSchema {
    fn from(session: Session) -> Self {
        AuditSessionSchema {
            id: session.id().to_string(),
            user_id: session.user_id(),
            device_info: session.device_info().to_string(),
            ip_address: session.ip_address().to_string(),
            created_at: session.created_at().to_string(),
            expires_at: session.expires_at().to_string(),
            revoked_at: session.revoked_at().map(|t| t.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuditResponseView {
    sessions: Vec<AuditSessionSchema>,
}

impl AuditResponseView {
    pub fn new(sessions: Vec<AuditSessionSchema>) -> Self {
        AuditResponseView { sessions }
    }

    pub fn sessions(&self) -> &[AuditSessionSchema] {
        &self.sessions
    }
}

impl Display for AuditResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AuditResponseView {{ sessions: {:?} }}", self.sessions)
    }
}

impl From<Vec<AuditSessionSchema>> for AuditResponseView {
    fn from(sessions: Vec<AuditSessionSchema>) -> Self {
        AuditResponseView { sessions }
    }
}
