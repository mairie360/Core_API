use crate::database::roles::get_roles::RoleQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Role {
    /// Identifiant du rôle.
    #[schema(example = 2)]
    id: u64,
    /// Nom technique du rôle, unique sur la plateforme.
    #[schema(example = "agent")]
    name: String,
    /// Description lisible du rôle. Chaîne vide si le rôle n'en a pas.
    #[schema(example = "Agent municipal")]
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
pub struct AdminGetRolesResultView {
    /// Tous les rôles définis sur la plateforme.
    roles: Vec<Role>,
}

impl From<Vec<RoleQueryResult>> for AdminGetRolesResultView {
    fn from(results: Vec<RoleQueryResult>) -> Self {
        Self {
            roles: results.into_iter().map(|r| r.into()).collect(),
        }
    }
}
