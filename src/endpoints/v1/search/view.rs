use crate::database::search::view::GlobalSearchItem;
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
    #[param(required = true, value_type = String, min_length = 2, max_length = 100, example = "dupont")]
    q: Option<String>,
    /// Maximum number of results, an integer from 1 to 20 (8 by default).
    #[param(value_type = Option<u64>, minimum = 1, maximum = 20, example = 8)]
    limit: Option<String>,
}

impl SearchQuery {
    /// The term, trimmed.
    pub fn term(&self) -> &str {
        self.q.as_deref().unwrap_or_default().trim()
    }

    /// The parsed `limit` (8 without one); `None` when it is not an integer. Both parameters are
    /// read as text so that a malformed value gets a `400` naming it, like the other rules.
    fn parsed_limit(&self) -> Option<u64> {
        self.limit
            .as_deref()
            .map_or(Some(DEFAULT_SEARCH_LIMIT), |limit| {
                limit.trim().parse().ok()
            })
    }

    /// The bounded `limit`, once [`Self::validate`] accepted it.
    pub fn limit(&self) -> u64 {
        self.parsed_limit().unwrap_or(DEFAULT_SEARCH_LIMIT)
    }
}

impl Validate for SearchQuery {
    fn validate(&self) -> Result<(), ValidationError> {
        let Some(raw) = self.q.as_deref() else {
            return Err(ValidationError::new("q", "is required"));
        };
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
        // On the raw value: `trim` would drop a leading or trailing newline.
        if raw.chars().any(char::is_control) {
            return Err(ValidationError::new(
                "q",
                "must not contain control characters",
            ));
        }
        if !self
            .parsed_limit()
            .is_some_and(|limit| (1..=MAX_SEARCH_LIMIT).contains(&limit))
        {
            return Err(ValidationError::new(
                "limit",
                &format!("must be an integer between 1 and {MAX_SEARCH_LIMIT}"),
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
pub struct GlobalSearchResultView {
    /// At most `limit` results; empty when nothing matches.
    pub items: Vec<GlobalSearchItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(q: &str, limit: Option<u64>) -> SearchQuery {
        SearchQuery {
            q: Some(q.to_string()),
            limit: limit.map(|limit| limit.to_string()),
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
        assert!(query("\nab", None).validate().is_err());
        assert!(query("ab\n", None).validate().is_err());
    }

    #[test]
    fn mair_1228_limit_is_8_by_default_and_from_1_to_20() {
        assert_eq!(query("ab", None).limit(), 8);
        assert!(query("ab", Some(20)).validate().is_ok());
        assert!(query("ab", Some(21)).validate().is_err());
        assert!(query("ab", Some(0)).validate().is_err());
        for malformed in ["abc", "-1", "2.5", ""] {
            let search = SearchQuery {
                q: Some("ab".to_string()),
                limit: Some(malformed.to_string()),
            };
            let error = search.validate().unwrap_err().to_string();
            assert!(error.contains("`limit`"), "{malformed}: {error}");
        }
    }

    #[test]
    fn mair_1228_a_missing_term_is_named() {
        let search = SearchQuery {
            q: None,
            limit: None,
        };
        assert!(search.validate().unwrap_err().to_string().contains("`q`"));
    }

    #[test]
    fn mair_1228_like_wildcards_match_themselves() {
        assert_eq!(like_escape("50%_a\\b"), "50\\%\\_a\\\\b");
        assert_eq!(like_escape("dupont"), "dupont");
    }
}
