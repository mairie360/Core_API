use crate::database::{
    admin::get_user::view::{AdminGetUserQueryResultView, RoleQueryResult, User},
    sessions::Session,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
struct SessionResultView {
    id: String,
    device_info: String,
    ip_address: String,
    created_at: String,
    expires_at: String,
    revoked_at: Option<String>,
}

impl From<&Session> for SessionResultView {
    fn from(value: &Session) -> Self {
        Self {
            id: value.id().to_string(),
            device_info: value.device_info().to_string(),
            ip_address: value.ip_address().to_string(),
            created_at: value.created_at().to_string(),
            expires_at: value.expires_at().to_string(),
            revoked_at: value.revoked_at().map(|t| t.to_string()),
        }
    }
}

#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
struct RoleResultView {
    id: i32,
    name: String,
    description: Option<String>,
}

impl From<&RoleQueryResult> for RoleResultView {
    fn from(value: &RoleQueryResult) -> Self {
        Self {
            id: value.id(),
            name: value.name().to_string(),
            description: value.description().map(|d| d.to_string()),
        }
    }
}

#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
pub struct GetUserResultView {
    user: User,
    roles: Vec<RoleResultView>,
    sessions: Vec<SessionResultView>,
}

impl Display for GetUserResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetUserResultView {{ user: {}, roles: {}, sessions: {} }}",
            self.user,
            self.roles.len(),
            self.sessions.len()
        )
    }
}

impl From<AdminGetUserQueryResultView> for GetUserResultView {
    fn from(value: AdminGetUserQueryResultView) -> Self {
        Self {
            user: value.user().clone(),
            roles: value.roles().into_iter().map(|r| r.into()).collect(),
            sessions: value.sessions().into_iter().map(|s| s.into()).collect(),
        }
    }
}
