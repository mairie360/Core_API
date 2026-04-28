use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

#[derive(Debug)]
pub struct PatchRoleQueryView {
    id: u64,
    name: Option<String>,
    description: Option<String>,
    can_be_deleted: Option<Option<bool>>,
}

impl PatchRoleQueryView {
    pub fn new(
        id: u64,
        name: Option<String>,
        description: Option<String>,
        can_be_deleted: Option<Option<bool>>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            can_be_deleted,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn can_be_deleted(&self) -> Option<Option<bool>> {
        self.can_be_deleted.clone()
    }
}

impl DatabaseQueryView for PatchRoleQueryView {
    fn get_request(&self) -> String {
        "UPDATE roles
         SET name = COALESCE($1, name),
             description = COALESCE($2, description),
             can_be_deleted = COALESCE($3, can_be_deleted)
         WHERE id = $4"
            .to_string()
    }
}

impl Display for PatchRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchRoleQueryView: id = {}, name = {:?}, description = {:?}, can_be_deleted = {:?}",
            self.id, self.name, self.description, self.can_be_deleted,
        )
    }
}
