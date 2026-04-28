use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RevokeRequestView {
    pub refresh_token: String,
}

impl RevokeRequestView {
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }
}

impl Display for RevokeRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokeRequestView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}
