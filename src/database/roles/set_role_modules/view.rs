use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Replaces the modules a role opens (MAIR-916) in one statement: the modules not listed are deleted,
/// the new ones inserted. The role is named by its id, or by its name right after `INSERT INTO roles`
/// (the creation does not return the id). Taking `admin` away from the `Admin` role is refused by the
/// trigger `tr_role_modules_protect_admin` (`23514`).
#[derive(serde::Deserialize)]
pub struct SetRoleModulesQueryView {
    by_name: bool,
    params: Vec<QueryParam>,
}

impl SetRoleModulesQueryView {
    /// Modules of the role `role_id`.
    #[must_use]
    pub fn for_id(role_id: u64, modules: &[String]) -> Self {
        Self {
            by_name: false,
            params: vec![
                QueryParam::I32(id_to_sql(role_id)),
                QueryParam::Text(crate::role_modules::joined(modules)),
            ],
        }
    }

    /// Modules of the role named `name` (just created).
    #[must_use]
    pub fn for_name(name: &str, modules: &[String]) -> Self {
        Self {
            by_name: true,
            params: vec![
                QueryParam::Text(name.to_string()),
                QueryParam::Text(crate::role_modules::joined(modules)),
            ],
        }
    }
}

impl ApiRequestDto for SetRoleModulesQueryView {
    fn query_sql(&self) -> &'static str {
        if self.by_name {
            "WITH r AS (SELECT id FROM roles WHERE name = $1), \
             wanted AS (SELECT unnest(string_to_array(NULLIF($2, ''), ',')) AS module), \
             removed AS (DELETE FROM role_modules rm USING r \
                         WHERE rm.role_id = r.id AND rm.module NOT IN (SELECT module FROM wanted)) \
             INSERT INTO role_modules (role_id, module) SELECT r.id, w.module FROM r, wanted w \
             ON CONFLICT DO NOTHING"
        } else {
            "WITH wanted AS (SELECT unnest(string_to_array(NULLIF($2, ''), ',')) AS module), \
             removed AS (DELETE FROM role_modules \
                         WHERE role_id = $1 AND module NOT IN (SELECT module FROM wanted)) \
             INSERT INTO role_modules (role_id, module) \
             SELECT r.id, w.module FROM roles r, wanted w WHERE r.id = $1 \
             ON CONFLICT DO NOTHING"
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for SetRoleModulesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetRoleModulesQueryView")
    }
}
