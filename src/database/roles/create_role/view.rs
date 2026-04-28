use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct CreateRoleQueryView {
    name: String,
    description: String,
    can_be_deleted: Option<bool>,
}

impl CreateRoleQueryView {
    pub fn new(name: &str, description: &str, can_be_deleted: Option<bool>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            can_be_deleted,
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

impl DatabaseQueryView for CreateRoleQueryView {
    fn get_request(&self) -> String {
        match self.can_be_deleted {
            // Si Some, on inclut la colonne et le paramètre $3
            Some(_) => "INSERT INTO roles (name, description, can_be_deleted)
                        VALUES ($1, $2, $3)"
                .to_string(),
            // Si None, on omet la colonne : Postgres appliquera son DEFAULT TRUE
            None => "INSERT INTO roles (name, description)
                     VALUES ($1, $2)"
                .to_string(),
        }
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
