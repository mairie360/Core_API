use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

pub struct GetGroupsQuerView {
    group_id: u64,
}

impl GetGroupsQuerView {
    pub fn new(group_id: u64) -> Self {
        Self { group_id }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl DatabaseQueryView for GetGroupsQuerView {
    fn get_request(&self) -> String {
        "SELECT * FROM groups WHERE id = $1".to_string()
    }
}

impl Display for GetGroupsQuerView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroups: group_id = {}", self.group_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Group {
    id: i32,
    owner_id: i32,
    name: String,
    description: String,
}

impl Group {
    pub fn new(id: i32, name: &str, owner_id: i32, description: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            owner_id,
            description: description.to_string(),
        }
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Group: id = {}, name = {}, owner_id = {}, description = {}",
            self.id, self.name, self.owner_id, self.description,
        )
    }
}
