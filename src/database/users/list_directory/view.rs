use crate::database::ids::id_to_sql_i64;
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
                QueryParam::I64(id_to_sql_i64(limit)),
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
    // The matches are collected first (`MATERIALIZED`), through the trigram indexes when there is
    // a search, then sorted and cut: written as one `ORDER BY ... LIMIT`, the generic plan of the
    // prepared query walked `idx_users_name_order` (Database 3.0.1) in order and filtered every
    // user on the way for a selective search (MAIR-477). The roles and groups are only aggregated
    // for the rows kept.
    fn query_sql(&self) -> &'static str {
        "WITH matched AS MATERIALIZED ( \
            SELECT u.id, u.first_name, u.last_name, u.email FROM users u \
            WHERE COALESCE(u.is_archived, false) = false \
              AND (NULLIF($1, '') IS NULL \
                OR u.first_name ILIKE '%' || $1 || '%' \
                OR u.last_name ILIKE '%' || $1 || '%' \
                OR (u.first_name || ' ' || u.last_name) ILIKE '%' || $1 || '%' \
                OR u.email ILIKE '%' || $1 || '%') \
              AND (NULLIF($2, '') IS NULL \
                OR u.id = ANY(string_to_array($2, ',')::int[])) \
              AND (NULLIF($3, '') IS NULL OR EXISTS ( \
                SELECT 1 FROM group_members scoped WHERE scoped.user_id = u.id \
                  AND scoped.group_id = ANY(string_to_array($3, ',')::int[]))) \
         ), page AS ( \
            SELECT * FROM matched ORDER BY last_name, first_name, id LIMIT $4 \
         ) \
         SELECT to_jsonb(t) FROM ( \
            SELECT p.id, p.first_name, p.last_name, p.email, \
                COALESCE( \
                    (SELECT jsonb_agg(DISTINCT r.name) FROM user_roles ur \
                     JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = p.id), \
                    '[]'::jsonb) AS roles, \
                COALESCE( \
                    (SELECT jsonb_agg(DISTINCT gm.group_id) FROM group_members gm \
                     WHERE gm.user_id = p.id), \
                    '[]'::jsonb) AS group_ids \
            FROM page p \
            ORDER BY p.last_name, p.first_name, p.id \
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

/// Fiche d'annuaire d'un utilisateur actif : identité, rôles et groupes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct DirectoryUser {
    /// Identifiant de l'utilisateur, à réutiliser dans `GET /api/v1/user/{id}/`.
    #[schema(example = 1)]
    pub id: i32,
    /// Prénom de l'utilisateur.
    #[schema(example = "Jean")]
    pub first_name: String,
    /// Nom de famille de l'utilisateur.
    #[schema(example = "Dupont")]
    pub last_name: String,
    /// Adresse e-mail, unique sur la plateforme.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    pub email: String,
    /// Noms des rôles portés par l'utilisateur. Un compte créé sans rôle reçoit `Guest` par défaut.
    #[schema(example = json!(["agent"]))]
    pub roles: Vec<String>,
    /// Identifiants des groupes dont l'utilisateur est membre. Vide s'il n'appartient à aucun.
    #[schema(example = json!([3, 7]))]
    pub group_ids: Vec<i32>,
}
