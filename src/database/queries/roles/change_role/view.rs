use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct ChangeRoleQueryView {
    id: u64,
    name: String,
    description: String,
    can_be_deleted: Option<bool>,
}

impl ChangeRoleQueryView {
    pub fn new(id: u64, name: &str, description: &str, can_be_deleted: Option<bool>) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            can_be_deleted,
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

impl DatabaseQueryView for ChangeRoleQueryView {
    fn get_request(&self) -> String {
        "UPDATE roles
         SET name = COALESCE($1, name),
             description = COALESCE($2, description),
             can_be_deleted = COALESCE($3, can_be_deleted)
         WHERE id = $4
         RETURNING *"
            .to_string()
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
