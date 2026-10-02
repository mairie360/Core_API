use crate::database::ids::id_to_sql_i64;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Partial update of a role: only the fields given are written.
///
/// Static SQL text, see `PatchUserQueryView` (MAIR-390).
#[derive(Debug, serde::Deserialize)]
pub struct PatchRoleQueryView {
    id: u64,
    name: Option<String>,
    description: Option<String>,
    // Deliberate double Option: tells "absent" (None), "sent as null" (Some(None)) and "sent with
    // a value" (Some(Some(_))) apart.
    #[allow(clippy::option_option)]
    can_be_deleted: Option<Option<bool>>,
    params: Vec<QueryParam>,
}

impl PatchRoleQueryView {
    #[must_use]
    pub fn new(
        id: u64,
        name: Option<String>,
        description: Option<String>,
        can_be_deleted: Option<Option<bool>>,
    ) -> Self {
        let params = vec![
            QueryParam::Bool(name.is_some()),
            QueryParam::Text(name.clone().unwrap_or_default()),
            QueryParam::Bool(description.is_some()),
            QueryParam::Text(description.clone().unwrap_or_default()),
            QueryParam::Bool(can_be_deleted.is_some()),
            // `QueryParam` cannot bind a NULL bool: a separate flag asks for NULL.
            QueryParam::Bool(matches!(can_be_deleted, Some(None))),
            QueryParam::Bool(can_be_deleted.flatten().unwrap_or_default()),
            QueryParam::I64(id_to_sql_i64(id)),
        ];

        Self {
            id,
            name,
            description,
            can_be_deleted,
            params,
        }
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[must_use]
    pub const fn can_be_deleted(&self) -> Option<Option<bool>> {
        self.can_be_deleted
    }

    /// True when no field was given: there is nothing to write.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        self.name.is_none() && self.description.is_none() && self.can_be_deleted.is_none()
    }
}

impl ApiRequestDto for PatchRoleQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE roles SET \
            name = CASE WHEN $1 THEN $2 ELSE name END, \
            description = CASE WHEN $3 THEN $4 ELSE description END, \
            can_be_deleted = CASE WHEN NOT $5 THEN can_be_deleted WHEN $6 THEN NULL ELSE $7 END \
         WHERE id = $8"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchRoleQueryView: id = {}, name = {:?}, description = {:?}, can_be_deleted = {:?}",
            self.id, self.name, self.description, self.can_be_deleted,
        )
    }
}
