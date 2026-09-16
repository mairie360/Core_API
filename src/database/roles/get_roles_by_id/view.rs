use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetRolesByIdQueryView {
    id: Vec<i32>,
}

impl GetRolesByIdQueryView {
    #[must_use]
    pub const fn new(id: Vec<i32>) -> Self {
        Self { id }
    }

    #[must_use]
    pub fn id(&self) -> &[i32] {
        &self.id
    }
}

impl ApiRequestDto for GetRolesByIdQueryView {
    fn query_sql(&self) -> &'static str {
        // La liste d'IDs varie à chaque appel : `ANY($1)` n'est pas exprimable avec les variantes
        // actuelles de `QueryParam` (pas de type "tableau"), donc on construit le tableau Postgres
        // directement dans le texte (IDs entiers uniquement, donc pas d'injection possible) et on
        // "leak" pour obtenir un &'static str, comme l'exige `ApiRequestDto`.
        let ids = if self.id.is_empty() {
            "ARRAY[]::int[]".to_string()
        } else {
            format!(
                "ARRAY[{}]",
                self.id
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };

        Box::leak(
            format!(
                "SELECT row_to_json(t) FROM (SELECT name, description, created_at, updated_at, can_be_deleted FROM roles WHERE id = ANY({ids})) t"
            )
            .into_boxed_str(),
        )
    }

    fn query_params(&self) -> &[QueryParam] {
        &[]
    }
}

impl Display for GetRolesByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetRolesByIdQueryView: id={:?}", self.id)
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Role {
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    can_be_deleted: bool,
}

impl Role {
    #[must_use]
    pub const fn new(
        name: String,
        description: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        can_be_deleted: bool,
    ) -> Self {
        Self {
            name,
            description,
            created_at,
            updated_at,
            can_be_deleted,
        }
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
            "Role: name={}, description={:?}, created_at={}, updated_at={:?}, can_be_deleted={}",
            self.name, self.description, self.created_at, self.updated_at, self.can_be_deleted
        )
    }
}
