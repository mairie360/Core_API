use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Whether renaming role `id` to `name` would hit the `protect_role_names` trigger: system roles
/// (`can_be_deleted = FALSE`) keep their name. `false` when the name is unchanged.
#[derive(serde::Deserialize)]
pub struct IsRenameForbiddenQueryView {
    id: u64,
    name: String,
    params: Vec<QueryParam>,
}

impl IsRenameForbiddenQueryView {
    #[must_use]
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            params: vec![
                QueryParam::Text(name.to_string()),
                QueryParam::I64(id as i64),
            ],
        }
    }
}

impl ApiRequestDto for IsRenameForbiddenQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT NOT can_be_deleted AND name <> $1 FROM roles WHERE id = $2"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsRenameForbiddenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsRenameForbiddenQueryView: id = {}, name = {}",
            self.id, self.name
        )
    }
}
