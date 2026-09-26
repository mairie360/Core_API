use crate::database::users::list_directory::DirectoryUser;
use crate::endpoints::validation::{
    check_opaque, check_optional, Validate, ValidationError, MAX_SEARCH_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub const MAX_DIRECTORY_LIMIT: u64 = 1000;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DirectoryUsersQuery {
    /// Recherche sur le prénom, le nom ou l'email. Insensible à la casse et partielle.
    #[param(max_length = 255, example = "dupont")]
    search: Option<String>,
    /// Identifiants d'utilisateurs séparés par des virgules (ex. `1,2,3`). Un identifiant qui
    /// n'est pas un entier strictement positif fait échouer la requête en `400`.
    #[param(max_length = 255, example = "1,2,3")]
    ids: Option<String>,
    /// Ne garde que les membres d'au moins un de ces groupes (identifiants séparés par des
    /// virgules). Mêmes règles de validation que `ids`.
    #[param(max_length = 255, example = "3,7")]
    group_ids: Option<String>,
    /// Nombre maximal d'utilisateurs renvoyés, de 1 à 1000 (1000 par défaut). Hors de cet
    /// intervalle, la requête échoue en `400`.
    #[param(minimum = 1, maximum = 1000, example = 50)]
    limit: Option<u64>,
}

impl DirectoryUsersQuery {
    pub fn search(&self) -> Option<&str> {
        self.search.as_deref()
    }

    pub fn ids(&self) -> Option<&str> {
        self.ids.as_deref()
    }

    pub fn group_ids(&self) -> Option<&str> {
        self.group_ids.as_deref()
    }

    pub const fn limit(&self) -> Option<u64> {
        self.limit
    }
}

/// Analyse une liste « 1,2,3 » d'identifiants strictement positifs ; `None` si un élément est invalide.
pub fn parse_id_list(value: Option<&str>) -> Option<Vec<u64>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Some(Vec::new());
    };
    value
        .split(',')
        .map(|id| {
            id.trim()
                .parse::<u64>()
                .ok()
                .filter(|id| *id > 0 && i32::try_from(*id).is_ok())
        })
        .collect()
}

/// Résultat d'une recherche dans l'annuaire.
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct DirectoryUsersResultView {
    /// Utilisateurs non archivés correspondant aux filtres. Vide si aucun ne correspond.
    pub users: Vec<DirectoryUser>,
}

impl Validate for DirectoryUsersQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.search.as_deref(), |search| {
            check_opaque("search", search, MAX_SEARCH_LENGTH)
        })?;
        check_optional(self.ids.as_deref(), |ids| {
            check_opaque("ids", ids, MAX_SEARCH_LENGTH)
        })?;
        check_optional(self.group_ids.as_deref(), |ids| {
            check_opaque("group_ids", ids, MAX_SEARCH_LENGTH)
        })
    }
}
