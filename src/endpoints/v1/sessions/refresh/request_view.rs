use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequestView {
    refresh_token: String,
}

impl RefreshRequestView {
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }
}

impl Display for RefreshRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RefreshRequestView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}
