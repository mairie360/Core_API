use crate::database::admin::list_users::AdminUserRow;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub const DEFAULT_PAGE_SIZE: u64 = 20;
pub const MAX_PAGE_SIZE: u64 = 500;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AdminListUsersQuery {
    /// Page demandée, à partir de 1.
    page: Option<u64>,
    /// Taille de page, de 1 à 500 (20 par défaut).
    page_size: Option<u64>,
    /// Recherche sur le prénom, le nom ou l'email.
    search: Option<String>,
    /// Restreint la liste aux membres de ce groupe.
    group_id: Option<u64>,
}

impl AdminListUsersQuery {
    pub fn page(&self) -> Option<u64> {
        self.page
    }

    pub fn page_size(&self) -> Option<u64> {
        self.page_size
    }

    pub fn search(&self) -> Option<&str> {
        self.search.as_deref()
    }

    pub fn group_id(&self) -> Option<u64> {
        self.group_id
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminListUsersResultView {
    pub users: Vec<AdminUserRow>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
    pub total_pages: u64,
}
