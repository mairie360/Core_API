use crate::endpoints::validation::{
    check_description, check_label, Validate, ValidationError, MAX_DESCRIPTION_LENGTH,
    MAX_NAME_LENGTH,
};
use utoipa::ToSchema;

/// Group to create; the caller becomes its owner.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupView {
    /// Group name. Required, 1 to 64 characters, no control character.
    #[schema(min_length = 1, max_length = 64, example = "Service urbanisme")]
    name: String,
    /// Description of the group. Required on creation; pass an empty string if there is none.
    #[schema(max_length = 1000, example = "Building permit processing")]
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

/// Group created.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostGroupResultView {
    /// Id assigned to the created group, to reuse in `/api/v1/groups/{group_id}/`.
    #[schema(example = 3)]
    id: u64,
}

impl PostGroupResultView {
    pub const fn new(id: u64) -> Self {
        Self { id }
    }
}

impl Validate for PostGroupView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("name", &self.name, MAX_NAME_LENGTH)?;
        check_description("description", &self.description, MAX_DESCRIPTION_LENGTH)
    }
}
