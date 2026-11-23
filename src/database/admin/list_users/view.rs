use crate::database::ids::{id_to_sql, id_to_sql_i64};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

// Account state shown by the administration (MAIR-904), from the columns of `users u`: suspended by an
// administrator, archived (« Inactif »), invited (never signed in: no password yet or first
// connection pending) or active.
macro_rules! admin_account_status_sql {
    ($u:literal) => {
        concat!(
            "(CASE WHEN ",
            $u,
            ".status = 'suspended' THEN 'suspended' \
              WHEN COALESCE(",
            $u,
            ".is_archived, false) THEN 'inactive' \
              WHEN ",
            $u,
            ".password IS NULL OR COALESCE(",
            $u,
            ".first_connect, false) THEN 'invited' \
              ELSE 'active' END)"
        )
    };
}

// Filters shared by the list and the count, on `users u`. Parameters: $2 group id, $3 account
// status, $4 role id (NULL, or an empty status, = no filter); the search ($1) is applied by the
// caller.
macro_rules! admin_users_filters {
    ($u:literal) => {
        concat!(
            "($2::int IS NULL OR EXISTS ( \
                SELECT 1 FROM group_members gm WHERE gm.user_id = ",
            $u,
            ".id AND gm.group_id = $2)) \
             AND (NULLIF($3::text, '') IS NULL OR ",
            admin_account_status_sql!($u),
            " = $3) \
             AND ($4::int IS NULL OR EXISTS ( \
                SELECT 1 FROM user_roles fr WHERE fr.user_id = ",
            $u,
            ".id AND fr.role_id = $4))"
        )
    };
}

// Filter of a search: case-insensitive match on the full names (both orders) and the e-mail
// (`user_search_text_sql!`), or on the name of the agent's service (MAIR-904 F01, through the ids of the
// matching services so the trigram index of the names stays usable), plus the shared filters.
// $1 = search.
macro_rules! admin_users_search_filter {
    () => {
        concat!(
            "(",
            crate::user_search_text_sql!(),
            " LIKE '%' || lower($1) || '%' OR u.service_id = ANY(ARRAY( \
                SELECT sv.id FROM services sv WHERE lower(sv.name) LIKE '%' || lower($1) || '%'))) AND ",
            admin_users_filters!("u")
        )
    };
}

// Columns of a row of the list, read from `users u` for the ids of the page.
macro_rules! admin_users_row {
    ($order:literal) => {
        concat!(
        "SELECT u.id, u.first_name, u.last_name, u.email, u.phone_number, u.phone_country, \
            u.status, COALESCE(u.is_archived, false) AS is_archived, \
            ",
        admin_account_status_sql!("u"),
        " AS account_status, \
            COALESCE( \
                (SELECT jsonb_agg(jsonb_build_object('id', r.id, 'name', r.name) ORDER BY r.name) \
                 FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
                 WHERE ur.user_id = u.id), \
                '[]'::jsonb) AS roles, \
            (SELECT count(*)::int FROM sessions s WHERE s.user_id = u.id AND s.revoked_at IS NULL \
                AND (s.expires_at IS NULL OR s.expires_at > now())) AS active_sessions, \
            (SELECT to_char(max(s.created_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
                FROM sessions s WHERE s.user_id = u.id) AS last_login_at, \
            jsonb_build_object( \
                'methods', to_jsonb(array_remove(ARRAY[ \
                    CASE WHEN EXISTS (SELECT 1 FROM user_totp t \
                        WHERE t.user_id = u.id AND t.confirmed_at IS NOT NULL) THEN 'totp' END, \
                    CASE WHEN EXISTS (SELECT 1 FROM user_passkeys p WHERE p.user_id = u.id) \
                        THEN 'passkey' END, \
                    CASE WHEN EXISTS (SELECT 1 FROM user_backup_codes b \
                        WHERE b.user_id = u.id AND b.used_at IS NULL) THEN 'backup_codes' END \
                ], NULL)), \
                'last_verified_at', (SELECT to_char(max(v.at) AT TIME ZONE 'UTC', \
                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') FROM ( \
                    SELECT to_timestamp(t.last_used_step * 30) AS at FROM user_totp t \
                    WHERE t.user_id = u.id AND t.confirmed_at IS NOT NULL \
                    UNION ALL SELECT p.last_used_at FROM user_passkeys p WHERE p.user_id = u.id \
                    UNION ALL SELECT b.used_at FROM user_backup_codes b WHERE b.user_id = u.id) v) \
            ) AS two_factor, \
            (SELECT jsonb_build_object('id', sv.id, 'name', sv.name) FROM services sv \
                WHERE sv.id = u.service_id) AS service \
         FROM page JOIN users u ON u.id = page.id \
         ORDER BY ", $order
        )
    };
}

/// The trimmed search, `None` when empty.
fn search_term(search: Option<&str>) -> Option<String> {
    search
        .map(str::trim)
        .filter(|search| !search.is_empty())
        .map(str::to_string)
}

/// Filters of the administration list (MAIR-904), every one optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdminUsersFilter {
    /// Free search on the names and the e-mail.
    pub search: Option<String>,
    /// Members of this group only.
    pub group_id: Option<u64>,
    /// `active`, `invited`, `inactive` or `suspended` (see [`AdminUserRow::account_status`]).
    pub account_status: Option<String>,
    /// Holders of this role only.
    pub role_id: Option<u64>,
}

impl AdminUsersFilter {
    /// Parameters $1 to $4 of the list and the count.
    fn params(&self) -> Vec<QueryParam> {
        vec![
            // The lib has no optional text parameter: an empty text means « no filter ».
            QueryParam::Text(search_term(self.search.as_deref()).unwrap_or_default()),
            QueryParam::OptionI32(self.group_id.map(id_to_sql)),
            QueryParam::Text(self.account_status.clone().unwrap_or_default()),
            QueryParam::OptionI32(self.role_id.map(id_to_sql)),
        ]
    }

    fn searched(&self) -> bool {
        search_term(self.search.as_deref()).is_some()
    }
}

/// Page of users for the administration, roles included, sorted by last name, first name then id
/// (ascending, or descending with `descending`).
///
/// Two statements (MAIR-477, measured by the MAIR-474 load test): without a search, the ids of the
/// page are read in the order of `idx_users_name_order` (Database 3.0.1, walked backwards for the
/// descending order) instead of sorting every user (18-29 ms for the last page of 10 000 users,
/// 3 ms this way); with one, the matches are collected first through the trigram indexes
/// (`MATERIALIZED`), then sorted. In one statement, the generic plan of the prepared query walked
/// the name index for a selective search too (2 ms to 20 ms). In both, the roles are only
/// aggregated for the rows of the page; `role` is the first (and, since MAIR-564, only) one.
/// Parameters: $1 search, $2 group, $3 account status, $4 role, $5 limit, $6 offset.
#[derive(Debug, serde::Deserialize)]
pub struct AdminListUsersQueryView {
    params: Vec<QueryParam>,
    searched: bool,
    descending: bool,
}

impl AdminListUsersQueryView {
    /// Page of the users matching `search` (and members of `group_id`), ascending.
    #[must_use]
    pub fn new(search: Option<&str>, group_id: Option<u64>, page: u64, page_size: u64) -> Self {
        let filter = AdminUsersFilter {
            search: search.map(str::to_string),
            group_id,
            ..AdminUsersFilter::default()
        };
        Self::filtered(&filter, false, page, page_size)
    }

    /// Page of the users matching `filter`, by name ascending or `descending`.
    #[must_use]
    pub fn filtered(
        filter: &AdminUsersFilter,
        descending: bool,
        page: u64,
        page_size: u64,
    ) -> Self {
        let mut params = filter.params();
        params.push(QueryParam::I64(id_to_sql_i64(page_size)));
        params.push(QueryParam::I64(id_to_sql_i64(
            page.saturating_sub(1).saturating_mul(page_size),
        )));
        Self {
            params,
            searched: filter.searched(),
            descending,
        }
    }
}

macro_rules! admin_users_page_sql {
    (searched, $order:literal, $row_order:literal) => {
        concat!(
            "WITH matched AS MATERIALIZED ( \
                SELECT u.id, u.first_name, u.last_name FROM users u WHERE ",
            admin_users_search_filter!(),
            " ), page AS ( \
                SELECT id FROM matched ORDER BY ",
            $order,
            " LIMIT $5 OFFSET $6) \
             SELECT to_jsonb(t) || jsonb_build_object('role', t.roles -> 0) FROM (",
            admin_users_row!($row_order),
            ") t"
        )
    };
    (all, $order:literal, $row_order:literal) => {
        concat!(
            "WITH page AS ( \
                SELECT u.id FROM users u WHERE ",
            admin_users_filters!("u"),
            " ORDER BY ",
            $order,
            " LIMIT $5 OFFSET $6) \
             SELECT to_jsonb(t) || jsonb_build_object('role', t.roles -> 0) FROM (",
            admin_users_row!($row_order),
            ") t"
        )
    };
}

impl ApiRequestDto for AdminListUsersQueryView {
    fn query_sql(&self) -> &'static str {
        match (self.searched, self.descending) {
            (true, false) => admin_users_page_sql!(
                searched,
                "last_name, first_name, id",
                "u.last_name, u.first_name, u.id"
            ),
            (true, true) => admin_users_page_sql!(
                searched,
                "last_name DESC, first_name DESC, id DESC",
                "u.last_name DESC, u.first_name DESC, u.id DESC"
            ),
            (false, false) => admin_users_page_sql!(
                all,
                "u.last_name, u.first_name, u.id",
                "u.last_name, u.first_name, u.id"
            ),
            (false, true) => admin_users_page_sql!(
                all,
                "u.last_name DESC, u.first_name DESC, u.id DESC",
                "u.last_name DESC, u.first_name DESC, u.id DESC"
            ),
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
    /// Number of users matching `search` (and members of `group_id`).
    #[must_use]
    pub fn new(search: Option<&str>, group_id: Option<u64>) -> Self {
        Self::filtered(&AdminUsersFilter {
            search: search.map(str::to_string),
            group_id,
            ..AdminUsersFilter::default()
        })
    }

    /// Number of users matching `filter`.
    #[must_use]
    pub fn filtered(filter: &AdminUsersFilter) -> Self {
        Self {
            params: filter.params(),
            searched: filter.searched(),
        }
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
            concat!(
                "SELECT COUNT(*) FROM users u WHERE ",
                admin_users_filters!("u")
            )
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
    /// State of the account for the administration (MAIR-904): `suspended` (suspended by an
    /// administrator), `inactive` (archived), `invited` (never signed in: no password yet or first
    /// connection pending) or `active`. The list filters on it with `account_status`.
    #[schema(example = "active")]
    #[serde(default = "default_account_status")]
    pub account_status: String,
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
    /// Number of sessions of the user still active: neither revoked nor expired (MAIR-584). The
    /// administration shows the user as online when it is above 0.
    #[schema(example = 2)]
    #[serde(default)]
    pub active_sessions: i32,
    /// Opening date of the latest session of the user (its last sign-in), RFC 3339 in UTC, or
    /// `null` when the user never signed in.
    #[schema(example = "2026-09-16T08:42:11Z")]
    #[serde(default)]
    pub last_login_at: Option<String>,
    /// Second factor of the user (MAIR-824, security tab of the administration).
    #[serde(default)]
    pub two_factor: AdminUserTwoFactor,
    /// Service of the mairie the agent belongs to (MAIR-923), `null` when none.
    #[schema(required = true)]
    #[serde(default)]
    pub service: Option<AdminUserService>,
}

/// Service of an agent, in the administration (MAIR-923).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminUserService {
    /// Id of the service.
    #[schema(example = 3)]
    pub id: i32,
    /// Name of the service.
    #[schema(example = "Roads")]
    pub name: String,
}

/// Second factor of a user, as the administrators see it: never a secret, a code or a key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminUserTwoFactor {
    /// Methods the user has enrolled, among `totp` (authenticator application, confirmed),
    /// `passkey` (at least one passkey) and `backup_codes` (at least one unused backup code).
    /// Empty when the user has no second factor.
    #[schema(example = json!(["totp", "backup_codes"]))]
    pub methods: Vec<String>,
    /// Latest successful verification of a second factor (TOTP code, passkey sign-in or backup
    /// code), RFC 3339 in UTC, or `null` when none was ever verified.
    #[schema(example = "2026-09-16T08:42:11Z")]
    pub last_verified_at: Option<String>,
}

fn default_account_status() -> String {
    "active".to_string()
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
