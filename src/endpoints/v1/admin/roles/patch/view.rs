use crate::endpoints::validation::{
    check_description, check_label, check_optional, Validate, ValidationError,
    MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Partial update of a role: only the given fields are updated.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchView {
    /// New name of the role. Absent or `null` to leave it unchanged.
    #[schema(min_length = 1, max_length = 64, example = "agent")]
    name: Option<String>,
    /// New description. Absent or `null` to leave it unchanged.
    #[schema(
        max_length = 1000,
        example = "Municipal agent allowed to process the files"
    )]
    description: Option<String>,
    /// Omitted or `null`: the current value is kept. `false` protects the role against deletion
    /// and renaming for good: a system role cannot be set back to `true` (MAIR-678).
    #[schema(example = true)]
    #[allow(clippy::option_option)] // Tri-state on purpose: omitted, `null`, or a value.
    can_be_deleted: Option<Option<bool>>,
    /// New modules of the navigation the agents of the role reach (MAIR-916), the whole list. Absent
    /// or `null` to leave them unchanged. The `Admin` role always keeps `admin` (`403`).
    #[schema(example = json!(["dashboard", "projects", "calendar"]))]
    modules: Option<Vec<String>>,
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

    /// New modules, if given.
    #[must_use]
    pub fn modules(&self) -> Option<&[String]> {
        self.modules.as_deref()
    }
}

impl Validate for PatchView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.name.as_deref(), |name| {
            check_label("name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.description.as_deref(), |description| {
            check_description("description", description, MAX_DESCRIPTION_LENGTH)
        })?;
        self.modules.as_deref().map_or(Ok(()), |modules| {
            crate::role_modules::check_modules("modules", modules)
        })
    }
}
