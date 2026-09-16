use crate::database::roles::get_roles::RoleQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Role {
    /// Identifiant du rôle, à réutiliser dans `POST /api/v1/admin/users/{userId}/roles/`.
    #[schema(example = 1)]
    id: u64,
    /// Nom technique du rôle, unique sur la plateforme.
    #[schema(example = "admin")]
    name: String,
    /// Description lisible du rôle. Chaîne vide si le rôle n'en a pas.
    #[schema(example = "Administrateur de la plateforme")]
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
pub struct GetRolesResultView {
    /// Tous les rôles définis sur la plateforme.
    roles: Vec<Role>,
}

impl From<Vec<RoleQueryResult>> for GetRolesResultView {
    fn from(results: Vec<RoleQueryResult>) -> Self {
        Self {
            roles: results.into_iter().map(|r| r.into()).collect(),
        }
    }
}
