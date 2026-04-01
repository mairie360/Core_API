use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::endpoints::v1::sessions::view::SessionSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HistoryResponseView {
    sessions: Vec<SessionSchema>,
}

impl HistoryResponseView {
    pub fn new(sessions: Vec<SessionSchema>) -> Self {
        HistoryResponseView { sessions }
    }
}

impl Display for HistoryResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HistoryResponseView {{ sessions: {:?} }}", self.sessions)
    }
}

impl From<Vec<SessionSchema>> for HistoryResponseView {
    fn from(sessions: Vec<SessionSchema>) -> Self {
        HistoryResponseView { sessions }
    }
}
