use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, serde::Deserialize)]
pub struct GetRolesQueryView {
    params: Vec<QueryParam>,
}

impl ApiRequestDto for GetRolesQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT id, name, description, created_at, updated_at, can_be_deleted FROM roles) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleQueryResult {
    id: i32,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    can_be_deleted: bool,
}

impl RoleQueryResult {
    #[must_use]
    pub const fn new(
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

    #[must_use]
    pub const fn id(&self) -> i32 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[must_use]
    pub const fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    #[must_use]
    pub const fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.updated_at.as_ref()
    }

    #[must_use]
    pub const fn can_be_deleted(&self) -> bool {
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
