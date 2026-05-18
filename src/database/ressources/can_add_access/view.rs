use crate::endpoints::v1::ressources::AccessType;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct CanAddAccessQueryView {
    owner_id: u64,
    target_id: u64,
    ressource_id: u64,
    ressource_type: String,
    access_type: AccessType,
}

impl CanAddAccessQueryView {
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

    pub fn owner_id(&self) -> u64 {
        self.owner_id
    }

    pub fn target_id(&self) -> u64 {
        self.target_id
    }

    pub fn ressource_id(&self) -> u64 {
        self.ressource_id
    }

    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }

    pub fn access_type(&self) -> AccessType {
        self.access_type
    }
}

impl DatabaseQueryView for CanAddAccessQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1 AND owner_id = $2)",
            self.ressource_type
        )
        .to_string()
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
