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
    /// Restricts the list to one state of account (MAIR-904): `active`, `invited`, `inactive`
    /// (archived) or `suspended`, as `account_status` reads it. Any other value answers `400`.
    #[param(example = "suspended")]
    account_status: Option<String>,
    /// Restricts the list to the holders of this role (MAIR-904).
    #[param(example = 2)]
    role_id: Option<u64>,
    /// Order of the list (MAIR-904): `name` (last name, first name, ascending, the default) or
    /// `-name` (descending). Any other value answers `400`.
    #[param(example = "-name")]
    sort: Option<String>,
}

/// The states `account_status` accepts.
pub const ACCOUNT_STATUSES: [&str; 4] = ["active", "invited", "inactive", "suspended"];

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

    pub fn account_status(&self) -> Option<&str> {
        self.account_status.as_deref()
    }

    pub const fn role_id(&self) -> Option<u64> {
        self.role_id
    }

    /// Whether the list is sorted by name descending (`sort=-name`).
    pub fn descending(&self) -> bool {
        self.sort.as_deref() == Some("-name")
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
        })?;
        if let Some(status) = self.account_status.as_deref() {
            if !ACCOUNT_STATUSES.contains(&status) {
                return Err(ValidationError::new(
                    "account_status",
                    "must be active, invited, inactive or suspended",
                ));
            }
        }
        if let Some(sort) = self.sort.as_deref() {
            if sort != "name" && sort != "-name" {
                return Err(ValidationError::new("sort", "must be name or -name"));
            }
        }
        Ok(())
    }
}
