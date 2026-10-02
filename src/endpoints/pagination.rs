//! Bounded lists (MAIR-425): `limit` / `offset` query parameters shared by the list endpoints
//! whose size grows with the data (session history, groups, group members).

use crate::endpoints::validation::{Validate, ValidationError};
use serde::Deserialize;
use utoipa::IntoParams;

/// Items returned when `limit` is absent.
pub const DEFAULT_LIMIT: u64 = 100;
/// Largest `limit` accepted.
pub const MAX_LIMIT: u64 = 500;
/// Largest `offset` accepted.
pub const MAX_OFFSET: u64 = 1_000_000;

/// One page of a list.
#[derive(Debug, Default, Clone, Copy, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    /// Number of items to return, from 1 to 500 (100 by default). Outside this range the request
    /// answers `400`. A page shorter than `limit` is the last one.
    #[param(minimum = 1, maximum = 500, example = 100)]
    limit: Option<u64>,
    /// Number of items to skip, from 0 (default) to 1000000. Outside this range the request
    /// answers `400`.
    #[param(minimum = 0, maximum = 1_000_000, example = 0)]
    offset: Option<u64>,
}

impl PageQuery {
    #[must_use]
    pub const fn new(limit: u64, offset: u64) -> Self {
        Self {
            limit: Some(limit),
            offset: Some(offset),
        }
    }

    /// `limit`, or [`DEFAULT_LIMIT`].
    #[must_use]
    pub fn limit(&self) -> u64 {
        self.limit.unwrap_or(DEFAULT_LIMIT)
    }

    /// `offset`, or 0.
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.offset.unwrap_or_default()
    }
}

impl Validate for PageQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        if !(1..=MAX_LIMIT).contains(&self.limit()) {
            return Err(ValidationError::new("limit", "must be between 1 and 500"));
        }
        if self.offset() > MAX_OFFSET {
            return Err(ValidationError::new(
                "offset",
                "must be between 0 and 1000000",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{PageQuery, DEFAULT_LIMIT};
    use crate::endpoints::validation::Validate;

    #[test]
    fn defaults_and_bounds() {
        let page = PageQuery::default();
        assert_eq!((page.limit(), page.offset()), (DEFAULT_LIMIT, 0));
        assert!(page.validate().is_ok());
        assert!(PageQuery::new(500, 1_000_000).validate().is_ok());
        assert!(PageQuery::new(0, 0).validate().is_err());
        assert!(PageQuery::new(501, 0).validate().is_err());
        assert!(PageQuery::new(1, 1_000_001).validate().is_err());
    }
}
