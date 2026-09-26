use crate::endpoints::validation::{
    check_description, check_label, check_optional, Validate, ValidationError,
    MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Modification partielle d'un rôle : seuls les champs fournis sont mis à jour.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchView {
    /// Nouveau nom du rôle. Absent ou `null` pour ne pas y toucher.
    #[schema(min_length = 1, max_length = 64, example = "agent")]
    name: Option<String>,
    /// Nouvelle description. Absent ou `null` pour ne pas y toucher.
    #[schema(
        max_length = 1000,
        example = "Agent municipal habilité à instruire les dossiers"
    )]
    description: Option<String>,
    /// Doublement optionnel : omettre le champ laisse la valeur actuelle, alors que `null`
    /// l'efface. `false` protège le rôle contre sa suppression.
    #[schema(example = true)]
    #[allow(clippy::option_option)] // Tri-state on purpose: omitted, `null`, or a value.
    can_be_deleted: Option<Option<bool>>,
}

impl PatchView {
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    #[allow(clippy::option_option)]
    pub const fn can_be_deleted(&self) -> Option<Option<bool>> {
        self.can_be_deleted
    }
}

impl Validate for PatchView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.name.as_deref(), |name| {
            check_label("name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.description.as_deref(), |description| {
            check_description("description", description, MAX_DESCRIPTION_LENGTH)
        })
    }
}
