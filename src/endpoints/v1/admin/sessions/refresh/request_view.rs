use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequestView {
    pub session_id: String,
}

impl RefreshRequestView {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl Display for RefreshRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RefreshRequestView {{ session_id: {} }}",
            self.session_id
        )
    }
}
