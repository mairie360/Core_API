use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Roles whose id is in the list, ordered by id. Each row carries its id, so callers never pair
/// two lists by index (MAIR-390).
#[derive(serde::Deserialize)]
pub struct GetRolesByIdQueryView {
    id: Vec<i32>,
    params: Vec<QueryParam>,
}

impl GetRolesByIdQueryView {
    #[must_use]
    pub fn new(id: Vec<i32>) -> Self {
        // `QueryParam` has no array variant: the ids travel as one comma-separated text parameter
        // split back into an `int[]` by Postgres, so the SQL text stays a single static literal
        // (it used to be rebuilt and leaked on every call).
        let joined = id
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        Self {
            id,
            params: vec![QueryParam::Text(joined)],
        }
    }

    #[must_use]
    pub fn id(&self) -> &[i32] {
        &self.id
    }
}

impl ApiRequestDto for GetRolesByIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT r.id, r.name, r.description, r.created_at, r.updated_at, r.can_be_deleted, \
            COALESCE((SELECT array_agg(rm.module ORDER BY array_position(ARRAY['dashboard','projects','calendar','messages','training','admin','settings'], rm.module)) FROM role_modules rm WHERE rm.role_id = r.id), '{}') AS modules \
            FROM roles r \
            WHERE r.id = ANY(string_to_array(NULLIF($1, ''), ',')::int[]) \
            ORDER BY r.id\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetRolesByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetRolesByIdQueryView: id={:?}", self.id)
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Role {
    id: i32,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    can_be_deleted: bool,
    /// Modules the role opens (MAIR-916): read by `GetRolesOfUserQueryView` only, empty otherwise.
    #[serde(default)]
    modules: Vec<String>,
}

impl Role {
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
            modules: Vec::new(),
        }
    }

    /// Modules the role opens, in the order of the navigation (MAIR-916).
    #[must_use]
    pub fn modules(&self) -> Vec<String> {
        crate::role_modules::ordered(self.modules.clone())
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

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Role: id={}, name={}, description={:?}, created_at={}, updated_at={:?}, can_be_deleted={}",
            self.id,
            self.name, self.description, self.created_at, self.updated_at, self.can_be_deleted
        )
    }
}
