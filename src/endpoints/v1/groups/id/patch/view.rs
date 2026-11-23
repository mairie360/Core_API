use crate::endpoints::validation::{
    check_description, check_label, check_optional, Validate, ValidationError,
    MAX_DESCRIPTION_LENGTH,
};
use serde::Deserialize;
use utoipa::ToSchema;

/// `groups.name` is `VARCHAR(64)`.
pub const MAX_GROUP_NAME_LENGTH: usize = crate::endpoints::validation::MAX_NAME_LENGTH;

/// Partial update of a group: only the given fields are updated.
#[allow(clippy::option_option)]
#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchGroupView {
    /// New group name. Absent or `null` to leave it unchanged. Once trimmed, 1 to 64 characters,
    /// no control character.
    #[schema(min_length = 1, max_length = 64, example = "Urban planning department")]
    name: Option<String>,
    /// New description. Absent or `null` to leave it unchanged.
    #[schema(
        max_length = 1000,
        example = "Building permits and prior declarations processing"
    )]
    description: Option<String>,
    /// New colour of the group card (MAIR-923): `blue`, `teal`, `green`, `orange`, `purple`,
    /// `pink` or `neutral`. Absent keeps it, `null` gives back the default colour.
    #[serde(
        default,
        deserialize_with = "crate::endpoints::v1::user::me::nullable::double_option"
    )]
    #[schema(value_type = Option<String>, nullable, example = "teal")]
    color: Option<Option<String>>,
}

impl PatchGroupView {
    /// What the request does to the colour: absent keeps it, `Some(None)` clears it.
    #[allow(clippy::option_option)]
    pub fn color(&self) -> Option<Option<&str>> {
        self.color.as_ref().map(Option::as_deref)
    }

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
        })?;
        crate::endpoints::v1::groups::color::check_color(self.color().flatten())
    }
}
