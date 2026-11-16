use crate::database::ids::{id_to_sql, id_to_sql_i64};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

// Filter of a search, shared by the list and the count: case-insensitive match on the full
// names (both orders) and the e-mail (`user_search_text_sql!`), optionally restricted to the
// members of a group. $1 = search, $2 = group id (NULL = every user).
macro_rules! admin_users_search_filter {
    () => {
        concat!(
            crate::user_search_text_sql!(),
            " LIKE '%' || lower($1) || '%' \
            AND ($2::int IS NULL OR EXISTS ( \
                SELECT 1 FROM group_members gm WHERE gm.user_id = u.id AND gm.group_id = $2))"
        )
    };
}

// Columns of a row of the list, read from `users u` for the ids of the page.
macro_rules! admin_users_row {
    () => {
        "SELECT u.id, u.first_name, u.last_name, u.email, u.phone_number, u.phone_country, \
            u.status, COALESCE(u.is_archived, false) AS is_archived, \
            COALESCE( \
                (SELECT jsonb_agg(jsonb_build_object('id', r.id, 'name', r.name) ORDER BY r.name) \
                 FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
                 WHERE ur.user_id = u.id), \
                '[]'::jsonb) AS roles \
         FROM page JOIN users u ON u.id = page.id \
         ORDER BY u.last_name, u.first_name, u.id"
    };
}

/// The trimmed search, `None` when empty.
fn search_term(search: Option<&str>) -> Option<String> {
    search
        .map(str::trim)
        .filter(|search| !search.is_empty())
        .map(str::to_string)
}

/// Page of users for the administration, roles included, sorted by last name, first name then id.
///
/// Two statements (MAIR-477, measured by the MAIR-474 load test): without a search, the ids of the
/// page are read in the order of `idx_users_name_order` (Database 3.0.1) instead of sorting every
/// user (18-29 ms for the last page of 10 000 users, 3 ms this way); with one, the matches are
/// collected first through the trigram indexes (`MATERIALIZED`), then sorted. In one statement,
/// the generic plan of the prepared query walked the name index for a selective search too (2 ms
/// to 20 ms). In both, the roles are only aggregated for the rows of the page; `role` is the first
/// (and, since MAIR-564, only) one.
#[derive(Debug, serde::Deserialize)]
pub struct AdminListUsersQueryView {
    params: Vec<QueryParam>,
    searched: bool,
}

impl AdminListUsersQueryView {
    #[must_use]
    pub fn new(search: Option<&str>, group_id: Option<u64>, page: u64, page_size: u64) -> Self {
        let search = search_term(search);
        let searched = search.is_some();
        let mut params = Vec::with_capacity(4);
        if let Some(search) = search {
            params.push(QueryParam::Text(search));
        }
        params.push(QueryParam::OptionI32(group_id.map(id_to_sql)));
        params.push(QueryParam::I64(id_to_sql_i64(page_size)));
        params.push(QueryParam::I64(id_to_sql_i64(
            page.saturating_sub(1).saturating_mul(page_size),
        )));
        Self { params, searched }
    }
}

impl ApiRequestDto for AdminListUsersQueryView {
    fn query_sql(&self) -> &'static str {
        if self.searched {
            concat!(
                "WITH matched AS MATERIALIZED ( \
                    SELECT u.id, u.first_name, u.last_name FROM users u WHERE ",
                admin_users_search_filter!(),
                " ), page AS ( \
                    SELECT id FROM matched ORDER BY last_name, first_name, id \
                    LIMIT $3 OFFSET $4) \
                 SELECT to_jsonb(t) || jsonb_build_object('role', t.roles -> 0) FROM (",
                admin_users_row!(),
                ") t"
            )
        } else {
            concat!(
                "WITH page AS ( \
                    SELECT o.id FROM users o \
                    WHERE $1::int IS NULL OR EXISTS ( \
                        SELECT 1 FROM group_members gm WHERE gm.user_id = o.id AND gm.group_id = $1) \
                    ORDER BY o.last_name, o.first_name, o.id \
                    LIMIT $2 OFFSET $3) \
                 SELECT to_jsonb(t) || jsonb_build_object('role', t.roles -> 0) FROM (",
                admin_users_row!(),
                ") t"
            )
        }
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

/// Total number of users matching the same filter as `AdminListUsersQueryView`.
#[derive(Debug, serde::Deserialize)]
pub struct AdminCountUsersQueryView {
    params: Vec<QueryParam>,
    searched: bool,
}

impl AdminCountUsersQueryView {
    #[must_use]
    pub fn new(search: Option<&str>, group_id: Option<u64>) -> Self {
        let search = search_term(search);
        let searched = search.is_some();
        let mut params = Vec::with_capacity(2);
        if let Some(search) = search {
            params.push(QueryParam::Text(search));
        }
        params.push(QueryParam::OptionI32(group_id.map(id_to_sql)));
        Self { params, searched }
    }
}

impl ApiRequestDto for AdminCountUsersQueryView {
    fn query_sql(&self) -> &'static str {
        if self.searched {
            concat!(
                "SELECT COUNT(*) FROM users u WHERE ",
                admin_users_search_filter!()
            )
        } else {
            "SELECT COUNT(*) FROM users u \
             WHERE $1::int IS NULL OR EXISTS ( \
                SELECT 1 FROM group_members gm WHERE gm.user_id = u.id AND gm.group_id = $1)"
        }
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

/// Role held by a user, in the administration list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminUserRole {
    /// Id of the role.
    #[schema(example = 2)]
    pub id: i32,
    /// Technical name of the role.
    #[schema(example = "agent")]
    pub name: String,
}

/// Row of the administration list of users, archived accounts included.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminUserRow {
    /// Id of the user.
    #[schema(example = 42)]
    pub id: i32,
    /// First name of the user.
    #[schema(example = "Jean")]
    pub first_name: String,
    /// Last name of the user.
    #[schema(example = "Dupont")]
    pub last_name: String,
    /// E-mail address, unique on the platform.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    pub email: String,
    /// Phone number in E.164 (`+33612345678`), or `null` when the user has none. A legacy number
    /// that could not be attributed to a country (MAIR-480) is returned as stored, digits only,
    /// with `phone_country` `null`.
    #[schema(example = "+33612345678")]
    pub phone_number: Option<String>,
    /// ISO 3166-1 alpha-2 code of the phone number's country, or `null` when there is no phone.
    #[schema(example = "FR")]
    #[serde(default)]
    pub phone_country: Option<String>,
    /// Status of the account as stored in the database.
    #[schema(example = "active")]
    pub status: String,
    /// `true` if the account is archived. These accounts show up here but not in
    /// `GET /api/v1/user/`, and can no longer sign in.
    #[schema(example = false)]
    pub is_archived: bool,
    /// The user's role: a user holds one role (MAIR-564), an account created without one gets
    /// `Guest`. `null` when the user has none.
    #[schema(required = true)]
    #[serde(default)]
    pub role: Option<AdminUserRole>,
    /// Deprecated: `role` as a list, kept for the clients written when a user could hold several
    /// roles. At most one entry, empty when the user has none. Read `role` instead.
    #[schema(deprecated)]
    pub roles: Vec<AdminUserRole>,
}

impl AdminUserRow {
    /// Replaces the stored national number by its E.164 form (MAIR-480): the rows are read with
    /// the national number of `users.phone_number`, the API answers in E.164.
    #[must_use]
    pub fn with_e164_phone(mut self) -> Self {
        self.phone_number =
            crate::phone::display(self.phone_country.as_deref(), self.phone_number.as_deref());
        self
    }
}
