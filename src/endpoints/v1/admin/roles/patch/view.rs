use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Modification partielle d'un rôle : seuls les champs fournis sont mis à jour.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchView {
    /// Nouveau nom du rôle. Absent ou `null` pour ne pas y toucher.
    #[schema(example = "agent")]
    name: Option<String>,
    /// Nouvelle description. Absent ou `null` pour ne pas y toucher.
    #[schema(example = "Agent municipal habilité à instruire les dossiers")]
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
