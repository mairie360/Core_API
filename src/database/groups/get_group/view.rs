use crate::database::ids::id_to_sql;
use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use utoipa::ToSchema;

#[derive(serde::Deserialize)]
pub struct GetGroupQuerView {
    group_id: u64,
    params: Vec<QueryParam>,
}

impl GetGroupQuerView {
    #[must_use]
    pub fn new(group_id: u64) -> Self {
        Self {
            group_id,
            params: vec![QueryParam::I32(id_to_sql(group_id))],
        }
    }

    #[must_use]
    pub const fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl ApiRequestDto for GetGroupQuerView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT g.*, \
         (SELECT count(pg.group_id) FROM project_groups pg WHERE pg.group_id = g.id)::int AS project_count \
         FROM groups g WHERE g.id = $1) t"
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

/// Group of users (department, team…) and its owner.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct Group {
    /// Id of the group.
    #[schema(example = 3)]
    id: i32,
    /// Id of the user who owns the group: the only one, with an administrator,
    /// who can update or delete it.
    #[schema(example = 2)]
    owner_id: i32,
    /// Name of the group.
    #[schema(example = "Urban planning department")]
    name: String,
    /// Description of the group, or `null` if it has none.
    #[schema(example = "Building permit processing")]
    description: Option<String>,
    /// Colour of the group card, a name of the charter palette (`blue`, `teal`, `green`,
    /// `orange`, `purple`, `pink`, `neutral`), or `null` for the default one (MAIR-923).
    #[serde(default)]
    #[schema(example = "teal")]
    color: Option<String>,
    /// Number of projects linked to the group (Project API, MAIR-683).
    #[serde(default)]
    #[schema(example = 2)]
    project_count: i32,
}

impl Group {
    #[must_use]
    pub fn new(id: i32, name: &str, owner_id: i32, description: Option<&str>) -> Self {
        Self {
            id,
            name: name.to_string(),
            owner_id,
            description: description.map(std::string::ToString::to_string),
            color: None,
            project_count: 0,
        }
    }

    /// The same group with a colour.
    #[must_use]
    pub fn with_color(mut self, color: Option<&str>) -> Self {
        self.color = color.map(str::to_string);
        self
    }

    #[must_use]
    pub fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    #[must_use]
    pub const fn project_count(&self) -> i32 {
        self.project_count
    }

    #[must_use]
    pub const fn id(&self) -> i32 {
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
