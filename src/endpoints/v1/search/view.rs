use crate::database::search::view::SearchItem;
use crate::endpoints::validation::{Validate, ValidationError};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Shortest term once trimmed: one character would match nearly everything.
pub const MIN_SEARCH_TERM_LENGTH: usize = 2;
/// Longest term accepted.
pub const MAX_SEARCH_TERM_LENGTH: usize = 100;
/// Results returned without `limit`.
pub const DEFAULT_SEARCH_LIMIT: u64 = 8;
/// Most results one search may return.
pub const MAX_SEARCH_LIMIT: u64 = 20;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    /// Searched text, case-insensitive and partial; leading and trailing spaces are ignored. From
    /// 2 to 100 characters once trimmed, no control character. `%` and `_` match themselves.
    #[param(min_length = 2, max_length = 100, example = "dupont")]
    q: String,
    /// Maximum number of results, from 1 to 20 (8 by default).
    #[param(minimum = 1, maximum = 20, example = 8)]
    limit: Option<u64>,
}

impl SearchQuery {
    /// The term, trimmed.
    pub fn term(&self) -> &str {
        self.q.trim()
    }

    pub fn limit(&self) -> u64 {
        self.limit.unwrap_or(DEFAULT_SEARCH_LIMIT)
    }
}

impl Validate for SearchQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        let term = self.term();
        let length = term.chars().count();
        if length < MIN_SEARCH_TERM_LENGTH {
            return Err(ValidationError::new(
                "q",
                &format!("must be at least {MIN_SEARCH_TERM_LENGTH} characters"),
            ));
        }
        if length > MAX_SEARCH_TERM_LENGTH {
            return Err(ValidationError::new(
                "q",
                &format!("must be at most {MAX_SEARCH_TERM_LENGTH} characters"),
            ));
        }
        if term.chars().any(char::is_control) {
            return Err(ValidationError::new(
                "q",
                "must not contain control characters",
            ));
        }
        if !(1..=MAX_SEARCH_LIMIT).contains(&self.limit()) {
            return Err(ValidationError::new(
                "limit",
                &format!("must be between 1 and {MAX_SEARCH_LIMIT}"),
            ));
        }
        Ok(())
    }
}

/// `term` escaped for a `LIKE` pattern: `\`, `%` and `_` only match themselves.
pub fn like_escape(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len());
    for character in term.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// Results of a global search, best match first.
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct SearchResultView {
    /// At most `limit` results; empty when nothing matches.
    pub items: Vec<SearchItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(q: &str, limit: Option<u64>) -> SearchQuery {
        SearchQuery {
            q: q.to_string(),
            limit,
        }
    }

    #[test]
    fn mair_1228_a_term_needs_two_characters_once_trimmed() {
        assert!(query(" a ", None).validate().is_err());
        assert!(query("", None).validate().is_err());
        assert!(query(" ab ", None).validate().is_ok());
        assert_eq!(query(" ab ", None).term(), "ab");
    }

    #[test]
    fn mair_1228_a_term_is_at_most_100_characters_without_control_characters() {
        assert!(query(&"a".repeat(100), None).validate().is_ok());
        assert!(query(&"a".repeat(101), None).validate().is_err());
        assert!(query("ab\u{0}c", None).validate().is_err());
    }

    #[test]
    fn mair_1228_limit_is_8_by_default_and_from_1_to_20() {
        assert_eq!(query("ab", None).limit(), 8);
        assert!(query("ab", Some(20)).validate().is_ok());
        assert!(query("ab", Some(21)).validate().is_err());
        assert!(query("ab", Some(0)).validate().is_err());
    }

    #[test]
    fn mair_1228_like_wildcards_match_themselves() {
        assert_eq!(like_escape("50%_a\\b"), "50\\%\\_a\\\\b");
        assert_eq!(like_escape("dupont"), "dupont");
    }
}
