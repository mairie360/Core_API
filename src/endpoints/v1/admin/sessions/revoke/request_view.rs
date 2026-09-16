use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Session à révoquer, désignée par son identifiant.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RevokeRequestView {
    pub session_id: String,
}

impl RevokeRequestView {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}
