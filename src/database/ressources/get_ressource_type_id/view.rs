use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetRessourceTypeIdQueryView {
    ressource_type: String,
}

impl GetRessourceTypeIdQueryView {
    pub fn new(ressource_type: &str) -> Self {
        Self {
            ressource_type: ressource_type.to_string(),
        }
    }

    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

impl DatabaseQueryView for GetRessourceTypeIdQueryView {
    fn get_request(&self) -> String {
        "SELECT id FROM resources WHERE name = $1".to_string()
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
