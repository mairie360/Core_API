use crate::database::ids::id_from_sql;
use crate::database::roles::get_roles::RoleQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Role of the platform.
///
/// Published as `PublicRole`: the administration's `Role` (with `can_be_deleted` and `modules`) has the same Rust
/// name, and two schemas of one name leave only one of them in the contract.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(as = PublicRole)]
struct Role {
    /// Id of the role, to reuse in `POST /api/v1/admin/users/{userId}/roles/`.
    #[schema(example = 1)]
    id: u64,
    /// Technical name of the role, unique on the platform.
    #[schema(example = "admin")]
    name: String,
    /// Readable description of the role. Empty string if the role has none.
    #[schema(example = "Platform administrator")]
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

/// List of roles.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetRolesResultView {
    /// Every role defined on the platform.
    roles: Vec<Role>,
}

impl From<Vec<RoleQueryResult>> for GetRolesResultView {
    fn from(results: Vec<RoleQueryResult>) -> Self {
        Self {
            roles: results.into_iter().map(std::convert::Into::into).collect(),
        }
    }
}
