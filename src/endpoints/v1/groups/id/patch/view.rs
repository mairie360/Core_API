use serde::Deserialize;
use utoipa::ToSchema;

pub const MAX_GROUP_NAME_LENGTH: usize = 255;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchGroupView {
    /// Nouveau nom du groupe. Absent ou `null` pour ne pas y toucher. Une fois les espaces de
    /// bord retirés, il ne peut être ni vide ni plus long que 255 caractères.
    #[schema(min_length = 1, max_length = 255, example = "Service urbanisme")]
    name: Option<String>,
    /// Nouvelle description. Absent ou `null` pour ne pas y toucher.
    #[schema(example = "Instruction des permis de construire et des déclarations préalables")]
    description: Option<String>,
}

impl PatchGroupView {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
