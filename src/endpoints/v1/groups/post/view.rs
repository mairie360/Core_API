use utoipa::ToSchema;

/// Groupe à créer ; l'appelant en devient propriétaire.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupView {
    /// Nom du groupe. Obligatoire, au plus 255 caractères.
    #[schema(max_length = 255, example = "Service urbanisme")]
    name: String,
    /// Description du groupe. Obligatoire à la création ; passer une chaîne vide s'il n'y en a pas.
    #[schema(example = "Instruction des permis de construire")]
    description: String,
}

impl PostGroupView {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Groupe créé.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupResultView {
    /// Identifiant attribué au groupe créé, à réutiliser dans `/api/v1/groups/{group_id}/`.
    #[schema(example = 3)]
    id: u64,
}

impl PostGroupResultView {
    pub const fn new(id: u64) -> Self {
        Self { id }
    }
}
