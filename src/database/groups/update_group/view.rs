use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Met à jour le nom et/ou la description d'un groupe (un champ absent est conservé) et renvoie le
/// groupe modifié. Aucune ligne n'est renvoyée si le groupe n'existe pas.
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
                QueryParam::I32(group_id as i32),
            ],
        }
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
                description = CASE WHEN $3 THEN $4 ELSE description END \
            WHERE id = $5 \
            RETURNING id, owner_id, name, description \
         ) \
         SELECT to_jsonb(t) FROM t"
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
