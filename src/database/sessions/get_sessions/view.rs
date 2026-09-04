use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetSessionsQueryView {
    id: Vec<i32>,
}

impl GetSessionsQueryView {
    pub fn new(id: Vec<u64>) -> Self {
        Self {
            id: id.into_iter().map(|id| id as i32).collect(),
        }
    }

    pub fn id(&self) -> &[i32] {
        &self.id
    }
}

impl ApiRequestDto for GetSessionsQueryView {
    fn query_sql(&self) -> &'static str {
        // Cf. GetRolesByIdQueryView : `ANY($1)` avec une liste dynamique n'est pas représentable
        // avec les variantes actuelles de `QueryParam` (pas de type "tableau"), donc on construit
        // le tableau Postgres dans le texte (entiers uniquement, pas d'injection possible) et on
        // "leak" pour obtenir un &'static str.
        let ids = if self.id.is_empty() {
            "ARRAY[]::int[]".to_string()
        } else {
            format!(
                "ARRAY[{}]",
                self.id
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };

        Box::leak(
            format!(
                "SELECT row_to_json(t) FROM (SELECT * FROM sessions WHERE user_id = ANY({})) t",
                ids
            )
            .into_boxed_str(),
        )
    }

    fn query_params(&self) -> &[QueryParam] {
        &[]
    }
}

impl Display for GetSessionsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetSessionsQueryView: id = {:?}", self.id)
    }
}
