use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct ChangeRoleQueryView {
    id: u64,
    name: String,
    description: String,
    can_be_deleted: Option<bool>,
    params: Vec<QueryParam>,
}

impl ChangeRoleQueryView {
    pub fn new(id: u64, name: &str, description: &str, can_be_deleted: Option<bool>) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            can_be_deleted,
            params: match can_be_deleted {
                Some(can_be_deleted) => vec![
                    QueryParam::Text(name.to_string()),
                    QueryParam::Text(description.to_string()),
                    QueryParam::Bool(can_be_deleted),
                    QueryParam::I64(id as i64),
                ],
                None => vec![
                    QueryParam::Text(name.to_string()),
                    QueryParam::Text(description.to_string()),
                    QueryParam::I64(id as i64),
                ],
            },
        }
    }

    pub fn id(&self) -> u64 {
        self.id
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

impl ApiRequestDto for ChangeRoleQueryView {
    fn query_sql(&self) -> &'static str {
        match self.can_be_deleted {
            Some(_) => {
                "UPDATE roles
                 SET name = COALESCE($1, name),
                     description = COALESCE($2, description),
                     can_be_deleted = COALESCE($3, can_be_deleted)
                 WHERE id = $4"
            }
            None => {
                "UPDATE roles
                 SET name = COALESCE($1, name),
                     description = COALESCE($2, description)
                 WHERE id = $3"
            }
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ChangeRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChangeRoleQueryView: id = {}, name = {}, description = {}, can_be_deleted = {:?}",
            self.id, self.name, self.description, self.can_be_deleted,
        )
    }
}
