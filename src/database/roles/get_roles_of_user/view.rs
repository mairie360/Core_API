use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Roles held by a user, as [`crate::database::roles::get_roles_by_id::Role`] rows ordered by id.
///
/// One round trip instead of the role ids (`GetUserRolesQueryView`) then the roles
/// (`GetRolesByIdQueryView`): each one waits for a pool connection under load (MAIR-474).
#[derive(serde::Deserialize)]
pub struct GetRolesOfUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetRolesOfUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for GetRolesOfUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (\
            SELECT r.id, r.name, r.description, r.created_at, r.updated_at, r.can_be_deleted \
            FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
            WHERE ur.user_id = $1 \
            ORDER BY r.id\
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetRolesOfUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetRolesOfUserQueryView: user_id = {}", self.user_id)
    }
}
