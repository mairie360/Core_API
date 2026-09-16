use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Rôle à créer ou à remplacer entièrement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleWriteView {
    /// Nom technique du rôle, unique sur la plateforme.
    #[schema(example = "agent")]
    name: String,
    /// Description lisible du rôle. Passer une chaîne vide s'il n'y en a pas.
    #[schema(example = "Agent municipal")]
    description: String,
    /// `false` protège le rôle : `DELETE /api/v1/admin/roles/{id}` le refusera en `403`.
    /// Absent ou `null` laisse la base appliquer sa valeur par défaut.
    #[schema(example = true)]
    can_be_deleted: Option<bool>,
}

impl RoleWriteView {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub const fn can_be_deleted(&self) -> Option<bool> {
        self.can_be_deleted
    }
}
