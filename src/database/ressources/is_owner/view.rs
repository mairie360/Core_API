use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Resource types whose instances carry an `owner_id` column that `core_api` may read
/// (see `Devops/Database` `security/api_grants.sql`), with the matching ownership query.
///
/// The table name is never interpolated from the caller's input: only the resource types listed
/// here can be owned, any other one (users, roles, sessions, ...) is administered by admins only.
const OWNED_RESSOURCE_QUERIES: [(&str, &str); 2] = [
    (
        "groups",
        "SELECT EXISTS(SELECT 1 FROM groups WHERE id = $1 AND owner_id = $2)",
    ),
    (
        "events",
        "SELECT EXISTS(SELECT 1 FROM events WHERE id = $1 AND owner_id = $2)",
    ),
];

/// Query used for a resource type that has no owner: nobody owns its instances.
const NOT_OWNABLE_QUERY: &str = "SELECT false";

fn ownership_query(ressource_type: &str) -> Option<&'static str> {
    OWNED_RESSOURCE_QUERIES
        .iter()
        .find(|(name, _)| *name == ressource_type)
        .map(|(_, query)| *query)
}

#[derive(serde::Deserialize)]
pub struct IsOwnerQueryView {
    ressource_type: String,
    ressource_id: u64,
    owner_id: u64,
    params: Vec<QueryParam>,
}

impl IsOwnerQueryView {
    #[must_use]
    pub fn new(owner_id: u64, ressource_id: u64, ressource_type: &str) -> Self {
        let params = if Self::supports(ressource_type) {
            vec![
                QueryParam::I64(ressource_id as i64),
                QueryParam::I64(owner_id as i64),
            ]
        } else {
            Vec::new()
        };
        Self {
            owner_id,
            ressource_id,
            ressource_type: ressource_type.to_string(),
            params,
        }
    }

    /// Whether instances of `ressource_type` have an owner. The query of an unsupported type
    /// always answers `false`.
    #[must_use]
    pub fn supports(ressource_type: &str) -> bool {
        ownership_query(ressource_type).is_some()
    }

    #[must_use]
    pub const fn owner_id(&self) -> u64 {
        self.owner_id
    }

    #[must_use]
    pub const fn ressource_id(&self) -> u64 {
        self.ressource_id
    }

    #[must_use]
    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

impl ApiRequestDto for IsOwnerQueryView {
    fn query_sql(&self) -> &'static str {
        ownership_query(&self.ressource_type).unwrap_or(NOT_OWNABLE_QUERY)
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsOwnerQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsOwnerQueryView: owner_id = {}, ressource_id = {}, ressource_type = {}",
            self.owner_id, self.ressource_id, self.ressource_type
        )
    }
}
