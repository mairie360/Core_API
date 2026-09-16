use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::endpoints::v1::sessions::view::SessionSchema;

/// Historique des sessions de l'utilisateur connecté, révoquées et expirées comprises.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct HistoryResponseView {
    /// Toutes les sessions de l'utilisateur, expirées et révoquées comprises.
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
