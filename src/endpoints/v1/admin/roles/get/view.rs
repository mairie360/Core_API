use crate::database::roles::get_roles::RoleQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Role {
    id: u64,
    name: String,
    description: String,
}

impl From<RoleQueryResult> for Role {
    fn from(result: RoleQueryResult) -> Self {
        Self {
            id: result.id() as u64,
            name: result.name().to_string(),
            description: result.description().unwrap_or_default().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetResponseView {
    roles: Vec<Role>,
}

impl From<Vec<RoleQueryResult>> for GetResponseView {
    fn from(results: Vec<RoleQueryResult>) -> Self {
        Self {
            roles: results.into_iter().map(|r| r.into()).collect(),
        }
    }
}
