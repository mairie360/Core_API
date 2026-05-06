use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct AddAccessToUserQueryView {
    user_id: u64,
    ressource_type_id: u64,
    ressource_instance_id: u64,
    access_type_id: u64,
}

impl AddAccessToUserQueryView {
    pub fn new(
        user_id: u64,
        ressource_type_id: u64,
        ressource_instance_id: u64,
        access_type_id: u64,
    ) -> Self {
        Self {
            user_id,
            ressource_type_id,
            ressource_instance_id,
            access_type_id,
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn ressource_type_id(&self) -> u64 {
        self.ressource_type_id
    }

    pub fn ressource_instance_id(&self) -> u64 {
        self.ressource_instance_id
    }

    pub fn access_type_id(&self) -> u64 {
        self.access_type_id
    }
}

impl DatabaseQueryView for AddAccessToUserQueryView {
    fn get_request(&self) -> String {
        "INSERT INTO access_control (user_id, resource_id, resource_instance_id, permission_id) VALUES ($1, $2, $3, $4)".to_string()
    }
}

impl Display for AddAccessToUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddAccessToUser: user_id = {}, ressource_type_id = {}, ressource_instance_id = {}, access_type_id = {}",
            self.user_id,
            self.ressource_type_id,
            self.ressource_instance_id,
            self.access_type_id
        )
    }
}
