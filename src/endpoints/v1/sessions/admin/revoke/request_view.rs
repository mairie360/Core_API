use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
pub struct RevokePathParamRequestView {
    pub user_id: u64,
}

impl RevokePathParamRequestView {
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for RevokePathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RevokePathParamRequestView {{ user_id: {} }}",
            self.user_id
        )
    }
}
