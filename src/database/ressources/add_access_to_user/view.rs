use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct AddAccessToUserQueryView {
    user_id: u64,
    ressource_type_id: u64,
    ressource_instance_id: u64,
    access_type_id: u64,
    params: Vec<QueryParam>,
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
            params: vec![
                QueryParam::I64(user_id as i64),
                QueryParam::I64(ressource_type_id as i64),
                QueryParam::I64(ressource_instance_id as i64),
                QueryParam::I64(access_type_id as i64),
            ],
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

impl ApiRequestDto for AddAccessToUserQueryView {
    fn query_sql(&self) -> &'static str {
        "INSERT INTO access_control (user_id, resource_id, resource_instance_id, permission_id) VALUES ($1, $2, $3, $4)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
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
