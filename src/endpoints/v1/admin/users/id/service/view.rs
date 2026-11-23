use crate::endpoints::validation::{Validate, ValidationError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Service an administrator attaches an agent to (MAIR-923).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserServiceView {
    /// Id of the service (`GET /api/v1/admin/services/`). `null` or absent: the agent has no
    /// service any more.
    #[schema(nullable, example = 3)]
    #[serde(default)]
    service_id: Option<u64>,
}

impl UserServiceView {
    #[must_use]
    pub const fn service_id(&self) -> Option<u64> {
        self.service_id
    }
}

impl Validate for UserServiceView {
    fn validate(&self) -> Result<(), ValidationError> {
        Ok(())
    }
}
