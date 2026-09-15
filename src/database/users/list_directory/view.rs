use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Annuaire des utilisateurs non archivés (identité, email, rôles et groupes), trié par nom, prénom
/// puis identifiant. Tous les filtres sont facultatifs et cumulatifs.
#[derive(Debug, serde::Deserialize)]
pub struct ListDirectoryUsersQueryView {
    params: Vec<QueryParam>,
}

impl ListDirectoryUsersQueryView {
    /// `ids` / `group_ids` vides = pas de filtre ; `limit` est borné par l'appelant.
    pub fn new(search: Option<&str>, ids: &[u64], group_ids: &[u64], limit: u64) -> Self {
        Self {
            params: vec![
                QueryParam::Text(search.map(str::trim).unwrap_or_default().to_string()),
                QueryParam::Text(join_ids(ids)),
                QueryParam::Text(join_ids(group_ids)),
                QueryParam::I64(limit as i64),
            ],
        }
    }
}

// QueryParam ne porte pas de tableau : les identifiants sont transmis en texte « 1,2,3 » puis
// convertis en int[] côté SQL.
fn join_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

impl ApiRequestDto for ListDirectoryUsersQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(t) FROM ( \
            SELECT u.id, u.first_name, u.last_name, u.email, \
                COALESCE( \
                    (SELECT jsonb_agg(DISTINCT r.name) FROM user_roles ur \
                     JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = u.id), \
                    '[]'::jsonb) AS roles, \
                COALESCE( \
                    (SELECT jsonb_agg(DISTINCT gm.group_id) FROM group_members gm \
                     WHERE gm.user_id = u.id), \
                    '[]'::jsonb) AS group_ids \
            FROM users u \
            WHERE COALESCE(u.is_archived, false) = false \
              AND (NULLIF($1, '') IS NULL \
                OR u.first_name ILIKE '%' || $1 || '%' \
                OR u.last_name ILIKE '%' || $1 || '%' \
                OR concat_ws(' ', u.first_name, u.last_name) ILIKE '%' || $1 || '%' \
                OR u.email ILIKE '%' || $1 || '%') \
              AND (NULLIF($2, '') IS NULL \
                OR u.id = ANY(string_to_array($2, ',')::int[])) \
              AND (NULLIF($3, '') IS NULL OR EXISTS ( \
                SELECT 1 FROM group_members scoped WHERE scoped.user_id = u.id \
                  AND scoped.group_id = ANY(string_to_array($3, ',')::int[]))) \
            ORDER BY u.last_name, u.first_name, u.id \
            LIMIT $4 \
         ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ListDirectoryUsersQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListDirectoryUsersQueryView: {:?}", self.params)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct DirectoryUser {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub roles: Vec<String>,
    pub group_ids: Vec<i32>,
}
