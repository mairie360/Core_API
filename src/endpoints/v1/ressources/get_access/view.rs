use crate::database::ressources::get_access_by_ressource::Access;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetAccessResultView {
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
