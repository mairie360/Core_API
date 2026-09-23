use crate::database::admin::list_users::AdminUserRow;
use crate::endpoints::validation::{
    check_opaque, check_optional, Validate, ValidationError, MAX_SEARCH_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub const DEFAULT_PAGE_SIZE: u64 = 20;
pub const MAX_PAGE_SIZE: u64 = 500;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AdminListUsersQuery {
    /// Page demandée, à partir de 1. Une page au-delà de `total_pages` renvoie une liste vide.
    #[param(minimum = 1, example = 1)]
    page: Option<u64>,
    /// Taille de page, de 1 à 500 (20 par défaut). Hors de cet intervalle, la requête échoue
    /// en `400`.
    #[param(minimum = 1, maximum = 500, example = 20)]
    page_size: Option<u64>,
    /// Recherche sur le prénom, le nom ou l'email. Insensible à la casse et partielle.
    #[param(max_length = 255, example = "dupont")]
    search: Option<String>,
    /// Restreint la liste aux membres de ce groupe.
    #[param(example = 3)]
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

/// Page de la liste d'administration des utilisateurs.
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminListUsersResultView {
    /// Utilisateurs de la page demandée, archivés compris. Vide au-delà de la dernière page.
    pub users: Vec<AdminUserRow>,
    /// Page effectivement renvoyée, telle que demandée ou 1 par défaut.
    #[schema(example = 1)]
    pub page: u64,
    /// Taille de page appliquée, telle que demandée ou 20 par défaut.
    #[schema(example = 20)]
    pub page_size: u64,
    /// Nombre total d'utilisateurs correspondant aux filtres, toutes pages confondues.
    #[schema(example = 137)]
    pub total: u64,
    /// Nombre de pages, soit `total` divisé par `page_size`, arrondi au supérieur.
    #[schema(example = 7)]
    pub total_pages: u64,
}

impl Validate for AdminListUsersQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.search.as_deref(), |search| {
            check_opaque("search", search, MAX_SEARCH_LENGTH)
        })
    }
}
