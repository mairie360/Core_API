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
    #[must_use]
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

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    #[must_use]
    pub const fn ressource_type_id(&self) -> u64 {
        self.ressource_type_id
    }

    #[must_use]
    pub const fn ressource_instance_id(&self) -> u64 {
        self.ressource_instance_id
    }

    #[must_use]
    pub const fn access_type_id(&self) -> u64 {
        self.access_type_id
    }
}

impl ApiRequestDto for AddAccessToUserQueryView {
    fn query_sql(&self) -> &'static str {
        // Idempotent: the `uq_access_entry` constraint never matches a user entry (its
        // `group_id` is NULL and NULLs are distinct), so duplicates are skipped explicitly.
        "INSERT INTO access_control (user_id, resource_id, resource_instance_id, permission_id) \
         SELECT $1, $2, $3, $4 \
         WHERE NOT EXISTS (\
             SELECT 1 FROM access_control \
             WHERE user_id = $1 AND resource_id = $2 AND resource_instance_id = $3 AND permission_id = $4\
         )"
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
