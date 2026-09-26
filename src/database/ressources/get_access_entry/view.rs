use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The resource instance an `access_control` entry applies to.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccessEntry {
    id: i32,
    ressource_type: String,
    resource_instance_id: i32,
}

impl AccessEntry {
    #[must_use]
    pub const fn id(&self) -> i32 {
        self.id
    }

    #[must_use]
    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }

    #[must_use]
    pub const fn resource_instance_id(&self) -> i32 {
        self.resource_instance_id
    }
}

/// Reads the resource type and instance of one `access_control` entry, to authorize its removal.
/// Answers `DbError::NotFound` when the entry does not exist.
#[derive(serde::Deserialize)]
pub struct GetAccessEntryQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl GetAccessEntryQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I64(id as i64)],
        }
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

impl ApiRequestDto for GetAccessEntryQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT ac.id, res.name AS ressource_type, ac.resource_instance_id \
            FROM access_control ac JOIN resources res ON res.id = ac.resource_id \
            WHERE ac.id = $1\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetAccessEntryQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetAccessEntry: id = {}", self.id)
    }
}
