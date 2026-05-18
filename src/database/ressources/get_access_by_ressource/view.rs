use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::FromRow, ToSchema)]
pub struct Access {
    id: i32,
    user_id: Option<i32>,
    group_id: Option<i32>,
    resource_id: i32,
    resource_instance_id: i32,
    permission_id: i32,
}

impl Access {
    pub fn new(
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

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn user_id(&self) -> Option<i32> {
        self.user_id
    }

    pub fn group_id(&self) -> Option<i32> {
        self.group_id
    }

    pub fn resource_id(&self) -> i32 {
        self.resource_id
    }

    pub fn resource_instance_id(&self) -> i32 {
        self.resource_instance_id
    }

    pub fn permission_id(&self) -> i32 {
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

pub struct GetAccessByRessourceQueryView {
    resource_id: u64,
}

impl GetAccessByRessourceQueryView {
    pub fn new(resource_id: u64) -> Self {
        Self { resource_id }
    }

    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }
}

impl DatabaseQueryView for GetAccessByRessourceQueryView {
    fn get_request(&self) -> String {
        "SELECT * FROM access_control WHERE resource_instance_id = $1".to_string()
    }
}

impl Display for GetAccessByRessourceQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetAccessByRessource: resource_id = {}",
            self.resource_id,
        )
    }
}
