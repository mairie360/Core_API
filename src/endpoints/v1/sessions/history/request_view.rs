use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HistoryRequestView {
    user_id: u64,
}

impl HistoryRequestView {
    pub fn new(user_id: u64) -> Self {
        HistoryRequestView { user_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for HistoryRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HistoryRequestView {{ user_id: {}}}", self.user_id)
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetPathParamRequestView {
    pub user_id: u64,
}

impl GetPathParamRequestView {
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for GetPathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HistoryRequestView {{ user_id: {} }}", self.user_id)
    }
}
