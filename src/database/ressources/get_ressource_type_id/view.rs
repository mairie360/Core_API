use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetRessourceTypeIdQueryView {
    ressource_type: String,
    params: Vec<QueryParam>,
}

impl GetRessourceTypeIdQueryView {
    pub fn new(ressource_type: &str) -> Self {
        Self {
            ressource_type: ressource_type.to_string(),
            params: vec![QueryParam::Text(ressource_type.to_string())],
        }
    }

    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

impl ApiRequestDto for GetRessourceTypeIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT id FROM resources WHERE name = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetRessourceTypeIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetRessourceTypeId: ressource_type = {}",
            self.ressource_type,
        )
    }
}
