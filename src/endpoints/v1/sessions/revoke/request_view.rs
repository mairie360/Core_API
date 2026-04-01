use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RevokeRequestView {
    pub token_id: String,
}

impl RevokeRequestView {
    pub fn token_id(&self) -> &str {
        &self.token_id
    }
}

impl Display for RevokeRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RevokeRequestView {{ token_id: {} }}", self.token_id)
    }
}
