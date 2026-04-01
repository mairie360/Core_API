use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshPathParamRequestView {
    user_id: u64,
}

impl RefreshPathParamRequestView {
    pub fn new(user_id: u64) -> Self {
        RefreshPathParamRequestView { user_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for RefreshPathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RefreshPathParamRequestView {{ user_id: {}}}",
            self.user_id
        )
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutPathParamRequestView {
    pub user_id: u64,
}

impl AboutPathParamRequestView {
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AboutPathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RefreshPathParamRequestView {{ user_id: {} }}",
            self.user_id
        )
    }
}
