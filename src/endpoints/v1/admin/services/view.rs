use crate::database::ids::id_from_sql;
use crate::database::services::Service;
use crate::endpoints::validation::{check_label, Validate, ValidationError, MAX_NAME_LENGTH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Name of a service to create or rename.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServiceWriteView {
    /// Name of the service, unique whatever its case (`Roads` and `roads` are the same service).
    #[schema(min_length = 1, max_length = 64, example = "Roads")]
    name: String,
}

impl ServiceWriteView {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Validate for ServiceWriteView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("name", &self.name, MAX_NAME_LENGTH)
    }
}

/// A service of the mairie.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServiceView {
    /// Id of the service.
    #[schema(example = 3)]
    id: u64,
    /// Name of the service.
    #[schema(example = "Roads")]
    name: String,
    /// Creation date of the service (RFC 3339, UTC).
    #[schema(format = DateTime, example = "2026-11-20T08:00:00Z")]
    created_at: String,
    /// Number of active agents attached to the service.
    #[schema(example = 12)]
    agents: u64,
}

impl From<Service> for ServiceView {
    fn from(service: Service) -> Self {
        Self {
            id: id_from_sql(service.id()),
            name: service.name().to_string(),
            created_at: service
                .created_at()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            agents: u64::try_from(service.agents()).unwrap_or_default(),
        }
    }
}

/// Every service of the mairie, by name.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServicesView {
    /// The services, sorted by name.
    services: Vec<ServiceView>,
}

impl From<Vec<Service>> for ServicesView {
    fn from(services: Vec<Service>) -> Self {
        Self {
            services: services.into_iter().map(ServiceView::from).collect(),
        }
    }
}
