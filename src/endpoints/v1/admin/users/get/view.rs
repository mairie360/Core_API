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
    /// Requested page, from 1. A page past `total_pages` returns an empty list.
    #[param(minimum = 1, example = 1)]
    page: Option<u64>,
    /// Page size, from 1 to 500 (20 by default). Outside this range, the request fails
    /// with `400`.
    #[param(minimum = 1, maximum = 500, example = 20)]
    page_size: Option<u64>,
    /// Search on the first name, the last name or the e-mail. Case-insensitive and partial.
    #[param(max_length = 255, example = "dupont")]
    search: Option<String>,
    /// Restricts the list to the members of this group.
    #[param(example = 3)]
    group_id: Option<u64>,
}

impl AdminListUsersQuery {
    pub const fn page(&self) -> Option<u64> {
        self.page
    }

    pub const fn page_size(&self) -> Option<u64> {
        self.page_size
    }

    pub fn search(&self) -> Option<&str> {
        self.search.as_deref()
    }

    pub const fn group_id(&self) -> Option<u64> {
        self.group_id
    }
}

/// Page of the administration list of users.
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AdminListUsersResultView {
    /// Users of the requested page, archived ones included. Empty past the last page.
    pub users: Vec<AdminUserRow>,
    /// Page actually returned, as requested or 1 by default.
    #[schema(example = 1)]
    pub page: u64,
    /// Page size applied, as requested or 20 by default.
    #[schema(example = 20)]
    pub page_size: u64,
    /// Total number of users matching the filters, across all pages.
    #[schema(example = 137)]
    pub total: u64,
    /// Number of pages: `total` divided by `page_size`, rounded up.
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
