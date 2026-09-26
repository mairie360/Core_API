use crate::database::ressources::get_access_by_ressource::Access;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::{IntoParams, ToSchema};

/// Resource type of the instance whose accesses are listed.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetAccessQuery {
    /// Resource type, as named in the `resources` table (`groups`, `events`, `users`, `roles`,
    /// ...). Required: instance ids are only unique within one resource type.
    #[param(example = "groups")]
    ressource_type: String,
}

impl GetAccessQuery {
    #[must_use]
    pub fn new(ressource_type: &str) -> Self {
        Self {
            ressource_type: ressource_type.to_string(),
        }
    }

    #[must_use]
    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

/// ACL entries of one resource instance.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetAccessResultView {
    /// Entries ordered by `id`; empty when nobody was granted an access on the instance.
    accesses: Vec<Access>,
}

impl GetAccessResultView {
    #[must_use]
    pub const fn new(accesses: Vec<Access>) -> Self {
        Self { accesses }
    }

    #[must_use]
    pub fn accesses(&self) -> &[Access] {
        &self.accesses
    }
}

impl Display for GetAccessResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Accesses:")?;
        for access in &self.accesses {
            writeln!(f, "  {access}")?;
        }
        Ok(())
    }
}
