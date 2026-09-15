use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

// Filtre commun à la liste et au comptage : recherche insensible à la casse sur le prénom, le nom
// (dans les deux ordres) et l'email, et restriction facultative aux membres d'un groupe.
// $1 = recherche ('' = aucune), $2 = identifiant de groupe (NULL = tous les utilisateurs).
macro_rules! admin_users_filter {
    () => {
        "(NULLIF($1, '') IS NULL \
            OR u.first_name ILIKE '%' || $1 || '%' \
            OR u.last_name ILIKE '%' || $1 || '%' \
            OR concat_ws(' ', u.first_name, u.last_name) ILIKE '%' || $1 || '%' \
            OR concat_ws(' ', u.last_name, u.first_name) ILIKE '%' || $1 || '%' \
            OR u.email ILIKE '%' || $1 || '%') \
        AND ($2::int IS NULL OR EXISTS ( \
            SELECT 1 FROM group_members gm WHERE gm.user_id = u.id AND gm.group_id = $2))"
    };
}

fn filter_params(search: Option<&str>, group_id: Option<u64>) -> Vec<QueryParam> {
    vec![
        QueryParam::Text(search.map(str::trim).unwrap_or_default().to_string()),
        QueryParam::OptionI32(group_id.map(|id| id as i32)),
    ]
}

/// Page d'utilisateurs pour l'administration, rôles inclus, triée par nom, prénom puis identifiant.
#[derive(Debug, serde::Deserialize)]
pub struct AdminListUsersQueryView {
    params: Vec<QueryParam>,
}

impl AdminListUsersQueryView {
    pub fn new(search: Option<&str>, group_id: Option<u64>, page: u64, page_size: u64) -> Self {
        let mut params = filter_params(search, group_id);
        params.push(QueryParam::I64(page_size as i64));
        params.push(QueryParam::I64((page.saturating_sub(1) * page_size) as i64));
        Self { params }
    }
}

impl ApiRequestDto for AdminListUsersQueryView {
    fn query_sql(&self) -> &'static str {
        concat!(
            "SELECT to_jsonb(t) FROM ( \
                SELECT u.id, u.first_name, u.last_name, u.email, u.phone_number, u.status, \
                    COALESCE(u.is_archived, false) AS is_archived, \
                    COALESCE( \
                        (SELECT jsonb_agg(jsonb_build_object('id', r.id, 'name', r.name) ORDER BY r.name) \
                         FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
                         WHERE ur.user_id = u.id), \
                        '[]'::jsonb) AS roles \
                FROM users u \
                WHERE ",
            admin_users_filter!(),
            " ORDER BY u.last_name, u.first_name, u.id \
                LIMIT $3 OFFSET $4 \
            ) t"
        )
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for AdminListUsersQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdminListUsersQueryView: {:?}", self.params)
    }
}

/// Nombre total d'utilisateurs correspondant au même filtre que `AdminListUsersQueryView`.
#[derive(Debug, serde::Deserialize)]
pub struct AdminCountUsersQueryView {
    params: Vec<QueryParam>,
}

impl AdminCountUsersQueryView {
    pub fn new(search: Option<&str>, group_id: Option<u64>) -> Self {
        Self {
            params: filter_params(search, group_id),
        }
    }
}

impl ApiRequestDto for AdminCountUsersQueryView {
    fn query_sql(&self) -> &'static str {
        concat!("SELECT COUNT(*) FROM users u WHERE ", admin_users_filter!())
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for AdminCountUsersQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdminCountUsersQueryView: {:?}", self.params)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminUserRole {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminUserRow {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub status: String,
    pub is_archived: bool,
    pub roles: Vec<AdminUserRole>,
}
