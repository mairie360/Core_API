use crate::endpoints::validation::{
    check_description, check_label, check_optional, Validate, ValidationError,
    MAX_DESCRIPTION_LENGTH,
};
use serde::Deserialize;
use utoipa::ToSchema;

/// `groups.name` is `VARCHAR(64)`.
pub const MAX_GROUP_NAME_LENGTH: usize = crate::endpoints::validation::MAX_NAME_LENGTH;

/// Modification partielle d'un groupe : seuls les champs fournis sont mis à jour.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchGroupView {
    /// New group name. Absent or `null` to leave it unchanged. Once trimmed, 1 to 64 characters,
    /// no control character, no `<` or `>`.
    #[schema(min_length = 1, max_length = 64, example = "Service urbanisme")]
    name: Option<String>,
    /// Nouvelle description. Absent ou `null` pour ne pas y toucher.
    #[schema(
        max_length = 1000,
        example = "Instruction des permis de construire et des déclarations préalables"
    )]
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

impl Validate for PatchGroupView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.name.as_deref(), |name| {
            check_label("name", name.trim(), MAX_GROUP_NAME_LENGTH)
        })?;
        check_optional(self.description.as_deref(), |description| {
            check_description("description", description, MAX_DESCRIPTION_LENGTH)
        })
    }
}
