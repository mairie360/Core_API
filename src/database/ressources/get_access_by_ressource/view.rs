use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// One ACL entry: a user or a group holds a permission on a resource instance.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct Access {
    /// Entry id, to pass as `access_id` to `POST /api/v1/ressources/remove_access`.
    #[schema(example = 12)]
    id: i32,
    /// User holding the access; `null` when the entry is granted to a group.
    #[schema(example = 42, nullable)]
    user_id: Option<i32>,
    /// Group holding the access; `null` when the entry is granted to a user.
    #[schema(example = json!(null), nullable)]
    group_id: Option<i32>,
    /// Id of the resource type in the `resources` table.
    #[schema(example = 5)]
    resource_id: i32,
    /// Id of the resource instance.
    #[schema(example = 3)]
    resource_instance_id: i32,
    /// Id of the granted permission in the `permissions` table.
    #[schema(example = 21)]
    permission_id: i32,
}

impl Access {
    #[must_use]
    pub const fn new(
        id: i32,
        user_id: Option<i32>,
        group_id: Option<i32>,
        resource_id: i32,
        resource_instance_id: i32,
        permission_id: i32,
    ) -> Self {
        Self {
            id,
            user_id,
            group_id,
            resource_id,
            resource_instance_id,
            permission_id,
        }
    }

    #[must_use]
    pub const fn id(&self) -> i32 {
        self.id
    }

    #[must_use]
    pub const fn user_id(&self) -> Option<i32> {
        self.user_id
    }

    #[must_use]
    pub const fn group_id(&self) -> Option<i32> {
        self.group_id
    }

    #[must_use]
    pub const fn resource_id(&self) -> i32 {
        self.resource_id
    }

    #[must_use]
    pub const fn resource_instance_id(&self) -> i32 {
        self.resource_instance_id
    }

    #[must_use]
    pub const fn permission_id(&self) -> i32 {
        self.permission_id
    }
}

impl Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Access:")?;
        writeln!(f, "  id: {}", self.id)?;
        writeln!(f, "  user_id: {:?}", self.user_id)?;
        writeln!(f, "  group_id: {:?}", self.group_id)?;
        writeln!(f, "  resource_id: {}", self.resource_id)?;
        writeln!(f, "  resource_instance_id: {}", self.resource_instance_id)?;
        writeln!(f, "  permission_id: {}", self.permission_id)?;
        Ok(())
    }
}

/// Lists the `access_control` entries of one instance of one resource type.
#[derive(serde::Deserialize)]
pub struct GetAccessByRessourceQueryView {
    resource_id: u64,
    ressource_type: String,
    params: Vec<QueryParam>,
}

impl GetAccessByRessourceQueryView {
    #[must_use]
    pub fn new(resource_id: u64, ressource_type: &str) -> Self {
        Self {
            resource_id,
            ressource_type: ressource_type.to_string(),
            params: vec![
                QueryParam::I64(resource_id as i64),
                QueryParam::Text(ressource_type.to_string()),
            ],
        }
    }

    #[must_use]
    pub const fn resource_id(&self) -> u64 {
        self.resource_id
    }

    #[must_use]
    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

impl ApiRequestDto for GetAccessByRessourceQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT ac.* FROM access_control ac JOIN resources res ON res.id = ac.resource_id \
            WHERE ac.resource_instance_id = $1 AND res.name = $2 ORDER BY ac.id\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetAccessByRessourceQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetAccessByRessource: resource_id = {}, ressource_type = {}",
            self.resource_id, self.ressource_type,
        )
    }
}
