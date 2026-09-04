use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use utoipa::ToSchema;

#[derive(serde::Deserialize)]
pub struct GetGroupQuerView {
    group_id: u64,
    params: Vec<QueryParam>,
}

impl GetGroupQuerView {
    pub fn new(group_id: u64) -> Self {
        Self {
            group_id,
            params: vec![QueryParam::I32(group_id as i32)],
        }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl ApiRequestDto for GetGroupQuerView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT * FROM groups WHERE id = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetGroupQuerView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroups: group_id = {}", self.group_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct Group {
    id: i32,
    owner_id: i32,
    name: String,
    description: Option<String>,
}

impl Group {
    pub fn new(id: i32, name: &str, owner_id: i32, description: Option<&str>) -> Self {
        Self {
            id,
            name: name.to_string(),
            owner_id,
            description: description.map(|d| d.to_string()),
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Group: id = {}, name = {}, owner_id = {}, description = {:?}",
            self.id, self.name, self.owner_id, self.description,
        )
    }
}
