use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub struct GetRolesQueryView {}

impl DatabaseQueryView for GetRolesQueryView {
    fn get_request(&self) -> String {
        "SELECT id, name, description, created_at, updated_at, can_be_deleted FROM roles"
            .to_string()
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::FromRow)]
pub struct RoleQueryResult {
    id: i32,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    can_be_deleted: bool,
}

impl RoleQueryResult {
    pub fn new(
        id: i32,
        name: String,
        description: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        can_be_deleted: bool,
    ) -> Self {
        Self {
            id,
            name,
            description,
            created_at,
            updated_at,
            can_be_deleted,
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.updated_at.as_ref()
    }

    pub fn can_be_deleted(&self) -> bool {
        self.can_be_deleted
    }
}

impl Display for RoleQueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RoleQueryResult: id = {}, name = {}, description = {:?}, created_at = {}, updated_at = {:?}, can_be_deleted = {}",
            self.id, self.name, self.description, self.created_at, self.updated_at, self.can_be_deleted
        )
    }
}
