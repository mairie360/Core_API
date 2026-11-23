use crate::database::ids::id_to_sql;
use chrono::{DateTime, Utc};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Columns of a service, with the number of active agents attached to it.
macro_rules! service_columns {
    ($table:literal) => {
        concat!(
            "SELECT row_to_json(t) FROM (SELECT s.id, s.name, s.created_at, ",
            "(SELECT count(*) FROM users u WHERE u.service_id = s.id AND u.is_archived IS NOT TRUE)::int AS agents ",
            "FROM ",
            $table,
            " s"
        )
    };
}

/// A service of the mairie.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Service {
    id: i32,
    name: String,
    created_at: DateTime<Utc>,
    agents: i32,
}

impl Service {
    #[must_use]
    pub const fn id(&self) -> i32 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    /// Active agents attached to the service.
    #[must_use]
    pub const fn agents(&self) -> i32 {
        self.agents
    }
}

/// Every service, by name.
#[derive(Default, Deserialize)]
pub struct ListServicesQueryView {
    params: Vec<QueryParam>,
}

impl ApiRequestDto for ListServicesQueryView {
    fn query_sql(&self) -> &'static str {
        concat!(
            service_columns!("services"),
            " ORDER BY lower(s.name), s.id) t"
        )
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ListServicesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListServicesQueryView")
    }
}

/// Creates a service and returns it (a duplicate name, whatever its case, is a `23505`).
#[derive(Deserialize)]
pub struct CreateServiceQueryView {
    params: Vec<QueryParam>,
}

impl CreateServiceQueryView {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            params: vec![QueryParam::Text(name.trim().to_string())],
        }
    }
}

impl ApiRequestDto for CreateServiceQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH s AS (INSERT INTO services (name) VALUES ($1) RETURNING id, name, created_at) \
         SELECT row_to_json(t) FROM (SELECT s.id, s.name, s.created_at, 0 AS agents FROM s) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for CreateServiceQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CreateServiceQueryView")
    }
}

/// Renames a service and returns it; no row when it does not exist.
#[derive(Deserialize)]
pub struct UpdateServiceQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl UpdateServiceQueryView {
    #[must_use]
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            params: vec![
                QueryParam::I32(id_to_sql(id)),
                QueryParam::Text(name.trim().to_string()),
            ],
        }
    }
}

impl ApiRequestDto for UpdateServiceQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH s AS (UPDATE services SET name = $2 WHERE id = $1 RETURNING id, name, created_at) \
         SELECT row_to_json(t) FROM (SELECT s.id, s.name, s.created_at, \
         (SELECT count(*) FROM users u WHERE u.service_id = s.id AND u.is_archived IS NOT TRUE)::int AS agents \
         FROM s) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for UpdateServiceQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UpdateServiceQueryView: id = {}", self.id)
    }
}

/// Deletes a service and returns its id; its agents are left without one (`ON DELETE SET NULL`).
#[derive(Deserialize)]
pub struct DeleteServiceQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl DeleteServiceQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id_to_sql(id))],
        }
    }
}

impl ApiRequestDto for DeleteServiceQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM services WHERE id = $1 RETURNING to_jsonb(id)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DeleteServiceQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteServiceQueryView: id = {}", self.id)
    }
}

/// Attaches an active account to a service, or detaches it (`None`), and returns its id. No row:
/// the account is unknown or archived; an unknown service is a foreign key violation (`23503`).
#[derive(Deserialize)]
pub struct SetUserServiceQueryView {
    user_id: u64,
    service_id: Option<u64>,
    params: Vec<QueryParam>,
}

impl SetUserServiceQueryView {
    #[must_use]
    pub fn new(user_id: u64, service_id: Option<u64>) -> Self {
        Self {
            user_id,
            service_id,
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::OptionI32(service_id.map(id_to_sql)),
            ],
        }
    }
}

impl ApiRequestDto for SetUserServiceQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET service_id = $2 WHERE id = $1 AND is_archived IS NOT TRUE RETURNING to_jsonb(id)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for SetUserServiceQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SetUserServiceQueryView: user_id = {}, service_id = {:?}",
            self.user_id, self.service_id
        )
    }
}
