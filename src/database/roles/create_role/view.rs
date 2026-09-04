use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct CreateRoleQueryView {
    name: String,
    description: String,
    can_be_deleted: Option<bool>,
    params: Vec<QueryParam>,
}

impl CreateRoleQueryView {
    pub fn new(name: &str, description: &str, can_be_deleted: Option<bool>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            can_be_deleted,
            params: match can_be_deleted {
                Some(can_be_deleted) => vec![
                    QueryParam::Text(name.to_string()),
                    QueryParam::Text(description.to_string()),
                    QueryParam::Bool(can_be_deleted),
                ],
                None => vec![
                    QueryParam::Text(name.to_string()),
                    QueryParam::Text(description.to_string()),
                ],
            },
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn can_be_deleted(&self) -> Option<bool> {
        self.can_be_deleted
    }
}

impl ApiRequestDto for CreateRoleQueryView {
    fn query_sql(&self) -> &'static str {
        match self.can_be_deleted {
            // Si Some, on inclut la colonne et le paramètre $3
            Some(_) => {
                "INSERT INTO roles (name, description, can_be_deleted)
                        VALUES ($1, $2, $3)"
            }
            // Si None, on omet la colonne : Postgres appliquera son DEFAULT TRUE
            None => {
                "INSERT INTO roles (name, description)
                     VALUES ($1, $2)"
            }
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for CreateRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CreateRoleQueryView: name = {}, description = {}, can_be_deleted = {:?}",
            self.name, self.description, self.can_be_deleted,
        )
    }
}
