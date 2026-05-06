use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct IsOwnerQueryView {
    ressource_type: String,
    ressource_id: u64,
    owner_id: u64,
}

impl IsOwnerQueryView {
    pub fn new(owner_id: u64, ressource_id: u64, ressource_type: &str) -> Self {
        Self {
            owner_id,
            ressource_id,
            ressource_type: ressource_type.to_string(),
        }
    }

    pub fn owner_id(&self) -> u64 {
        self.owner_id
    }

    pub fn ressource_id(&self) -> u64 {
        self.ressource_id
    }

    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

impl DatabaseQueryView for IsOwnerQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1 AND owner_id = $2)",
            self.ressource_type
        )
        .to_string()
    }
}

impl Display for IsOwnerQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsOwnerQueryView: owner_id = {}, ressource_id = {}, ressource_type = {}",
            self.owner_id, self.ressource_id, self.ressource_type
        )
    }
}
