use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateGroupQueryView {
    owner_id: u64,
    name: String,
    description: String,
    params: Vec<QueryParam>,
}

impl CreateGroupQueryView {
    pub fn new(owner_id: u64, name: &str, description: &str) -> Self {
        Self {
            owner_id,
            name: name.to_string(),
            description: description.to_string(),
            params: vec![
                QueryParam::I32(owner_id as i32),
                QueryParam::Text(name.to_string()),
                QueryParam::Text(description.to_string()),
            ],
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

impl ApiRequestDto for CreateGroupQueryView {
    fn query_sql(&self) -> &'static str {
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, $3) RETURNING id"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
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
