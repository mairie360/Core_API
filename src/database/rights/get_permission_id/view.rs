use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionAction {
    Create,
    Read,
    Update,
    Delete,
    ReadAll,
    UpdateAll,
    DeleteAll,
    Error,
}

impl PermissionAction {
    fn to_string(&self) -> &str {
        match self {
            PermissionAction::Create => "create",
            PermissionAction::Read => "read",
            PermissionAction::Update => "update",
            PermissionAction::Delete => "delete",
            PermissionAction::ReadAll => "read_all",
            PermissionAction::UpdateAll => "update_all",
            PermissionAction::DeleteAll => "delete_all",
            PermissionAction::Error => "error",
        }
    }
}

impl From<String> for PermissionAction {
    fn from(s: String) -> Self {
        match s.as_str() {
            "create" => PermissionAction::Create,
            "read" => PermissionAction::Read,
            "update" => PermissionAction::Update,
            "delete" => PermissionAction::Delete,
            "read_all" => PermissionAction::ReadAll,
            "update_all" => PermissionAction::UpdateAll,
            "delete_all" => PermissionAction::DeleteAll,
            _ => PermissionAction::Error,
        }
    }
}

impl Display for PermissionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub struct GetPermissionIdQueryView {
    resource_id: u64,
    action: PermissionAction,
}

impl GetPermissionIdQueryView {
    pub fn new(resource_id: u64, action: PermissionAction) -> Self {
        Self {
            resource_id,
            action,
        }
    }

    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }

    pub fn action(&self) -> PermissionAction {
        self.action
    }
}

impl DatabaseQueryView for GetPermissionIdQueryView {
    fn get_request(&self) -> String {
        "SELECT id FROM permissions WHERE resource_id = $1 AND action = $2".to_string()
    }
}

impl Display for GetPermissionIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetPermissionIdQueryView: resource_id = {}, action = {}",
            self.resource_id, self.action
        )
    }
}
