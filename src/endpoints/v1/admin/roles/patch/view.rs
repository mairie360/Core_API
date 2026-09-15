use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchView {
    name: Option<String>,
    description: Option<String>,
    // Double Option volontaire : distingue "champ absent" (None), "champ fourni à null"
    // (Some(None)) et "champ fourni avec une valeur" (Some(Some(_))).
    #[allow(clippy::option_option)]
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
