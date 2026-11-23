use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Updates the name and/or the description of a group (an absent field is kept) and returns the
/// updated group. No row is returned if the group does not exist.
#[derive(serde::Deserialize)]
pub struct UpdateGroupQueryView {
    group_id: u64,
    params: Vec<QueryParam>,
}

impl UpdateGroupQueryView {
    #[must_use]
    pub fn new(group_id: u64, name: Option<&str>, description: Option<&str>) -> Self {
        Self {
            group_id,
            params: vec![
                QueryParam::Bool(name.is_some()),
                QueryParam::Text(name.unwrap_or_default().to_string()),
                QueryParam::Bool(description.is_some()),
                QueryParam::Text(description.unwrap_or_default().to_string()),
                QueryParam::I32(id_to_sql(group_id)),
                QueryParam::Bool(false),
                QueryParam::Text(String::new()),
            ],
        }
    }

    /// Also changes the colour: `None` keeps it, `Some(None)` clears it (MAIR-923).
    #[must_use]
    #[allow(clippy::option_option)]
    pub fn with_color(mut self, color: Option<Option<&str>>) -> Self {
        if let Some(color) = color {
            self.params[5] = QueryParam::Bool(true);
            self.params[6] = QueryParam::Text(color.unwrap_or_default().to_string());
        }
        self
    }

    #[must_use]
    pub const fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl ApiRequestDto for UpdateGroupQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS ( \
            UPDATE groups SET \
                name = CASE WHEN $1 THEN $2 ELSE name END, \
                description = CASE WHEN $3 THEN $4 ELSE description END, \
                color = CASE WHEN $6 THEN NULLIF($7, '') ELSE color END \
            WHERE id = $5 \
            RETURNING id, owner_id, name, description, color \
         ) \
         SELECT to_jsonb(t) || jsonb_build_object('project_count', \
            (SELECT count(pg.group_id) FROM project_groups pg WHERE pg.group_id = t.id)::int) \
         FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for UpdateGroupQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UpdateGroupQueryView: group_id = {}", self.group_id)
    }
}
