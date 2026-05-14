use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct CreateGroupQueryView {
    owner_id: u64,
    name: String,
    description: String,
}

impl CreateGroupQueryView {
    pub fn new(owner_id: u64, name: &str, description: &str) -> Self {
        Self {
            owner_id,
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    pub fn owner_id(&self) -> u64 {
        self.owner_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl DatabaseQueryView for CreateGroupQueryView {
    fn get_request(&self) -> String {
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, $3) RETURNING id"
            .to_string()
    }
}

impl Display for CreateGroupQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CreateGroupQueryView: owner_id = {}, name = {}, description = {}",
            self.owner_id, self.name, self.description
        )
    }
}
