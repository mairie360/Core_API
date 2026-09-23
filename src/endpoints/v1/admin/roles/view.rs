use crate::endpoints::validation::{
    check_description, check_label, Validate, ValidationError, MAX_DESCRIPTION_LENGTH,
    MAX_NAME_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Rôle à créer ou à remplacer entièrement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleWriteView {
    /// Nom technique du rôle, unique sur la plateforme.
    #[schema(min_length = 1, max_length = 64, example = "agent")]
    name: String,
    /// Description lisible du rôle. Passer une chaîne vide s'il n'y en a pas.
    #[schema(max_length = 1000, example = "Agent municipal")]
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

impl Validate for RoleWriteView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("name", &self.name, MAX_NAME_LENGTH)?;
        check_description("description", &self.description, MAX_DESCRIPTION_LENGTH)
    }
}
