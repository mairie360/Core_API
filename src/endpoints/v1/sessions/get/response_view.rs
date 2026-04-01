use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::endpoints::v1::sessions::view::SessionSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetResponseView {
    sessions: Vec<SessionSchema>,
}

impl GetResponseView {
    pub fn new(sessions: Vec<SessionSchema>) -> Self {
        GetResponseView { sessions }
    }

    pub fn sessions(&self) -> &[SessionSchema] {
        &self.sessions
    }
}

impl Display for GetResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetResponseView {{ sessions: {:?} }}", self.sessions)
    }
}

impl From<Vec<SessionSchema>> for GetResponseView {
    fn from(sessions: Vec<SessionSchema>) -> Self {
        GetResponseView { sessions }
    }
}
