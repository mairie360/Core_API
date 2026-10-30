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
    /// Search on the first name, last name or e-mail. Case-insensitive and partial; leading and
    /// trailing spaces are ignored.
    #[param(max_length = 255, example = "dupont")]
    search: Option<String>,
    /// Comma-separated user ids (e.g. `1,2,3`), order and repetitions ignored. An id that is not
    /// a strictly positive integer fails the request with `400`.
    #[param(max_length = 255, example = "1,2,3")]
    ids: Option<String>,
    /// Keeps only the members of at least one of these groups (comma-separated ids). Same
    /// validation rules as `ids`.
    #[param(max_length = 255, example = "3,7")]
    group_ids: Option<String>,
    /// Maximum number of users returned, from 1 to 1000 (1000 by default). Outside this range,
    /// the request fails with `400`.
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

/// Parses a `1,2,3` list of strictly positive ids; `None` if an item is invalid.
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

/// Result of a directory search.
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct DirectoryUsersResultView {
    /// Non-archived users matching the filters. Empty if none matches.
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
