use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::endpoints::v1::sessions::view::SessionSchema;

/// Active sessions of the signed-in user.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetSessionsResultView {
    /// Still valid sessions, newest first. Can be empty.
    sessions: Vec<SessionSchema>,
}

impl GetSessionsResultView {
    pub const fn new(sessions: Vec<SessionSchema>) -> Self {
        Self { sessions }
    }
}

impl Display for GetSessionsResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetSessionsResultView {{ sessions: {:?} }}",
            self.sessions
        )
    }
}

impl From<Vec<SessionSchema>> for GetSessionsResultView {
    fn from(sessions: Vec<SessionSchema>) -> Self {
        Self { sessions }
    }
}
