use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct AddRolesQueryView {
    role_id: u64,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl AddRolesQueryView {
    #[must_use]
    pub fn new(role_id: u64, user_id: u64) -> Self {
        Self {
            role_id,
            user_id,
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::I32(id_to_sql(role_id)),
            ],
        }
    }

    #[must_use]
    pub const fn role_id(&self) -> u64 {
        self.role_id
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for AddRolesQueryView {
    // The directory (`GET /api/v1/user/`) shows this change: drop its cached pages (MAIR-524).
    fn invalidated_read_caches(&self) -> &'static [&'static str] {
        crate::database::users::list_directory::INVALIDATES_DIRECTORY
    }

    // A user holds one role (MAIR-564, Database 3.1.0: `user_roles` is keyed by the user): the
    // grant replaces the role in place. Returns the granted role id, no row when the user already
    // held it. A user or role that does not exist is a foreign key violation.
    fn query_sql(&self) -> &'static str {
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) \
         ON CONFLICT (user_id) DO UPDATE SET role_id = EXCLUDED.role_id \
         WHERE user_roles.role_id IS DISTINCT FROM EXCLUDED.role_id \
         RETURNING to_jsonb(role_id)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for AddRolesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddRolesQueryView: role_id = {}, user_id = {}",
            self.role_id, self.user_id
        )
    }
}
