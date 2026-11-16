use crate::database::ids::id_from_sql;
use crate::database::roles::get_roles::RoleQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Role of the platform.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Role {
    /// Id of the role.
    #[schema(example = 2)]
    id: u64,
    /// Technical name of the role, unique on the platform.
    #[schema(example = "agent")]
    name: String,
    /// Readable description of the role. Empty string if the role has none.
    #[schema(example = "Agent municipal")]
    description: String,
}

impl From<RoleQueryResult> for Role {
    fn from(result: RoleQueryResult) -> Self {
        Self {
            id: id_from_sql(result.id()),
            name: result.name().to_string(),
            description: result.description().unwrap_or_default().to_string(),
        }
    }
}

/// List of the roles, administrator view.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminGetRolesResultView {
    /// Every role defined on the platform.
    roles: Vec<Role>,
}

impl From<Vec<RoleQueryResult>> for AdminGetRolesResultView {
    fn from(results: Vec<RoleQueryResult>) -> Self {
        Self {
            roles: results.into_iter().map(std::convert::Into::into).collect(),
        }
    }
}
