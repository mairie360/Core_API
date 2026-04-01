use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuditPathParamRequestView {
    pub user_id: u64,
}

impl AuditPathParamRequestView {
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AuditPathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AuditPathParamRequestView {{ user_id: {} }}",
            self.user_id
        )
    }
}
