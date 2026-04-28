use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RevokeRequestView {
    pub session_id: String,
}

impl RevokeRequestView {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}
