use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::database::queries::sessions::Session;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionSchema {
    id: String,
    device_info: String,
    ip_address: String,
    created_at: String,
    expires_at: String,
    revoked_at: Option<String>,
}

impl From<Session> for SessionSchema {
    fn from(session: Session) -> Self {
        SessionSchema {
            id: session.id().to_string(),
            device_info: session.device_info().to_string(),
            ip_address: session.ip_address().to_string(),
            created_at: session.created_at().to_string(),
            expires_at: session.expires_at().to_string(),
            revoked_at: session.revoked_at().map(|t| t.to_string()),
        }
    }
}
