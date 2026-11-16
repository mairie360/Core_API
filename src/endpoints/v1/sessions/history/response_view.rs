use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::endpoints::v1::sessions::view::SessionSchema;

/// History of the sessions of the signed-in user, revoked and expired ones included.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct HistoryResponseView {
    /// Every session of the user, expired and revoked ones included.
    sessions: Vec<SessionSchema>,
}

impl HistoryResponseView {
    pub const fn new(sessions: Vec<SessionSchema>) -> Self {
        Self { sessions }
    }
}

impl Display for HistoryResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HistoryResponseView {{ sessions: {:?} }}", self.sessions)
    }
}

impl From<Vec<SessionSchema>> for HistoryResponseView {
    fn from(sessions: Vec<SessionSchema>) -> Self {
        Self { sessions }
    }
}
