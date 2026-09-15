use crate::database::ressources::is_owner::IsOwnerQueryView;
use crate::endpoints::v1::ressources::AccessType;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use std::fmt::Display;

pub struct CanAddAccessQueryView {
    owner_id: u64,
    target_id: u64,
    ressource_id: u64,
    ressource_type: String,
    access_type: AccessType,
}

impl CanAddAccessQueryView {
    #[must_use]
    pub fn new(
        owner_id: u64,
        target_id: u64,
        ressource_id: u64,
        ressource_type: &str,
        access_type: AccessType,
    ) -> Self {
        Self {
            owner_id,
            target_id,
            ressource_id,
            ressource_type: ressource_type.to_string(),
            access_type,
        }
    }

    #[must_use]
    pub const fn owner_id(&self) -> u64 {
        self.owner_id
    }

    #[must_use]
    pub const fn target_id(&self) -> u64 {
        self.target_id
    }

    #[must_use]
    pub const fn ressource_id(&self) -> u64 {
        self.ressource_id
    }

    #[must_use]
    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }

    #[must_use]
    pub const fn access_type(&self) -> AccessType {
        self.access_type
    }

    /// # Errors
    ///
    /// Retourne une erreur si la requête vers la base de données échoue.
    pub async fn check(&self, smart_db: &SmartDatabase) -> Result<bool, ApiLibError> {
        if self.access_type() == AccessType::Error {
            return Ok(false);
        }
        let is_owner: bool = smart_db
            .fetch_scalar(&IsOwnerQueryView::new(
                self.owner_id(),
                self.ressource_id(),
                self.ressource_type(),
            ))
            .await?;
        Ok(is_owner)
    }
}

impl Display for CanAddAccessQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CanAddAccess: owner_id = {}, target_id = {}, ressource_id = {}, ressource_type = {}, access_type = {}",
            self.owner_id,
            self.target_id,
            self.ressource_id,
            self.ressource_type,
            self.access_type.as_str()
        )
    }
}
