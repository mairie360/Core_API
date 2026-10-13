use crate::database::ids::id_to_sql_i64;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use mairie360_api_lib::smart_db::{ReadCache, ReadCacheKey};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Time to live of a cached directory page, in seconds (MAIR-524).
pub const DIRECTORY_CACHE_TTL_SECONDS: u64 = 60;

/// Redis read cache of the directory (`GET /api/v1/user/`, MAIR-524).
///
/// The page does not depend on the caller (no ACL: every authenticated user gets the same rows
/// for the same filters), so one entry per parameter set is shared by everyone
/// ([`ReadCache::shared`]). Every write view changing a user, a role assignment, a role name or a
/// group membership lists it in `invalidated_read_caches` ([`INVALIDATES_DIRECTORY`]).
pub const DIRECTORY_CACHE: ReadCache = ReadCache::new("directory", DIRECTORY_CACHE_TTL_SECONDS);

/// What the write views changing the directory return from `invalidated_read_caches`.
pub const INVALIDATES_DIRECTORY: &[&str] = &[DIRECTORY_CACHE.namespace()];

/// Directory of the non-archived users (identity, e-mail, roles and groups), sorted by last name,
/// first name then id. Every filter is optional and they add up.
#[derive(Debug, serde::Deserialize)]
pub struct ListDirectoryUsersQueryView {
    params: Vec<QueryParam>,
}

impl ListDirectoryUsersQueryView {
    /// Empty `ids` / `group_ids` = no filter; `limit` is bounded by the caller.
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

    /// The [`DIRECTORY_CACHE`] entry of these filters.
    ///
    /// Only normalisations the query itself ignores are applied, so two keys are equal only for
    /// requests returning the same rows: the search is trimmed (the SQL receives it trimmed, an
    /// empty search is no search), and `ids` / `group_ids` are sorted and deduplicated (they are
    /// matched with `= ANY(...)`, order and repetitions do not matter). The case of the search is
    /// kept: Postgres lowers it, and Rust and Postgres may disagree on some characters.
    pub fn cache_key(
        search: Option<&str>,
        ids: &[u64],
        group_ids: &[u64],
        limit: u64,
    ) -> ReadCacheKey {
        let search = search.map(str::trim).unwrap_or_default();
        DIRECTORY_CACHE.shared(&(search, sorted_ids(ids), sorted_ids(group_ids), limit))
    }
}

fn sorted_ids(ids: &[u64]) -> Vec<u64> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}

// QueryParam carries no array: the ids are sent as the text `1,2,3` and converted to int[] in
// SQL.
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
        concat!(
            "WITH matched AS MATERIALIZED ( \
            SELECT u.id, u.first_name, u.last_name, u.email FROM users u \
            WHERE COALESCE(u.is_archived, false) = false \
              AND (NULLIF($1, '') IS NULL OR ",
            crate::user_search_text_sql!(),
            " LIKE '%' || lower($1) || '%') \
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
        )
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

/// Directory entry of an active user: identity, roles and groups.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct DirectoryUser {
    /// Id of the user, reusable in `GET /api/v1/user/{id}/`.
    #[schema(example = 1)]
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
    /// Names of the roles held by the user. An account created without a role gets `Guest` by
    /// default. Empty if the user holds no role (never `null`).
    #[schema(example = json!(["agent"]))]
    pub roles: Vec<String>,
    /// Ids of the groups the user is a member of. Empty if they belong to none (never `null`).
    #[schema(example = json!([3, 7]))]
    pub group_ids: Vec<i32>,
}

#[cfg(test)]
mod tests {
    use super::ListDirectoryUsersQueryView as View;

    #[test]
    fn equivalent_filters_share_one_cache_entry() {
        let key = View::cache_key(Some("dupont"), &[3, 1, 2], &[7, 3], 50);
        assert_eq!(
            key,
            View::cache_key(Some("  dupont "), &[1, 2, 3, 3], &[3, 7], 50)
        );
        assert_eq!(
            View::cache_key(None, &[], &[], 50),
            View::cache_key(Some(" "), &[], &[], 50)
        );
    }

    #[test]
    fn different_filters_never_share_an_entry() {
        let key = View::cache_key(Some("dupont"), &[1], &[3], 50);
        assert_ne!(key, View::cache_key(Some("Dupont"), &[1], &[3], 50));
        assert_ne!(key, View::cache_key(Some("dupon"), &[1], &[3], 50));
        assert_ne!(key, View::cache_key(Some("dupont"), &[1, 2], &[3], 50));
        assert_ne!(key, View::cache_key(Some("dupont"), &[], &[1, 3], 50));
        assert_ne!(key, View::cache_key(Some("dupont"), &[3], &[1], 50));
        assert_ne!(key, View::cache_key(Some("dupont"), &[1], &[3], 51));
    }
}
