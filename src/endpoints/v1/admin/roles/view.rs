use crate::endpoints::validation::{
    check_description, check_label, Validate, ValidationError, MAX_DESCRIPTION_LENGTH,
    MAX_NAME_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Role to create or to replace entirely.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleWriteView {
    /// Technical name of the role, unique on the platform.
    #[schema(min_length = 1, max_length = 64, example = "agent")]
    name: String,
    /// Readable description of the role. Pass an empty string if there is none.
    #[schema(max_length = 1000, example = "Agent municipal")]
    description: String,
    /// `false` protects the role: `DELETE /api/v1/admin/roles/{id}` refuses it with `403`.
    /// Absent or `null` lets the database apply its default value.
    #[schema(example = true)]
    can_be_deleted: Option<bool>,
    /// Modules of the navigation the agents of the role reach (MAIR-916), among `dashboard`, `projects`,
    /// `calendar`, `messages`, `training`, `admin` and `settings`. Absent or `null`: on creation the role
    /// opens the dashboard only; on replacement its modules stay unchanged. The `Admin` role always keeps
    /// `admin` (`403`).
    #[schema(example = json!(["dashboard", "projects"]))]
    modules: Option<Vec<String>>,
}

impl RoleWriteView {
    /// Modules given in the body, if any.
    #[must_use]
    pub fn modules(&self) -> Option<&[String]> {
        self.modules.as_deref()
    }

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
        check_description("description", &self.description, MAX_DESCRIPTION_LENGTH)?;
        self.modules.as_deref().map_or(Ok(()), |modules| {
            crate::role_modules::check_modules("modules", modules)
        })
    }
}
