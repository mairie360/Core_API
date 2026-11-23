//! Password policy of the accounts (MAIR-757).
//!
//! It follows the CNIL 2022 recommendation for a password used alone: at least [`MIN_LENGTH`] characters with no character-class rule, not one
//! of the most common passwords, and not built on the account's own identity.
//!
//! [`crate::endpoints::validation::check_password`] applies the length and the common-password
//! rules to every route that sets a password; the identity rule needs the account and is applied
//! by [`identity_fragment`] where the handler knows it.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Minimum number of characters (Unicode scalar values) of a new password.
pub const MIN_LENGTH: usize = 12;

/// The 10,000 most common passwords of the `SecLists` list
/// (`Passwords/Common-Credentials/10k-most-common.txt`, MIT), lowercased and deduplicated.
/// Embedded in the binary: the check never calls an external service.
static COMMON_PASSWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    include_str!("common_passwords.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
});

/// Identity fragments shorter than this are not looked for (a two-letter first name would
/// refuse too many passwords).
const MIN_FRAGMENT_LENGTH: usize = 3;

/// Whether `password` is one of the most common passwords, whatever its case.
#[must_use]
pub fn is_common(password: &str) -> bool {
    COMMON_PASSWORDS.contains(password.to_lowercase().as_str())
}

/// What the password policy knows about the account a password is set for.
#[derive(Debug, Clone, Copy)]
pub struct Identity<'a> {
    pub email: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
}

/// The part of the account's identity that `password` contains, whatever the case: `"e-mail"`
/// (the local part of the address), `"first name"` or `"last name"`. `None` when it contains none.
#[must_use]
pub fn identity_fragment(password: &str, identity: Identity<'_>) -> Option<&'static str> {
    let password = password.to_lowercase();
    let local_part = identity
        .email
        .rsplit_once('@')
        .map_or(identity.email, |(local, _)| local);
    [
        (local_part, "e-mail"),
        (identity.first_name, "first name"),
        (identity.last_name, "last name"),
    ]
    .into_iter()
    .find(|(fragment, _)| {
        let fragment = fragment.trim().to_lowercase();
        fragment.chars().count() >= MIN_FRAGMENT_LENGTH && password.contains(&fragment)
    })
    .map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JEAN: Identity<'static> = Identity {
        email: "jean.dupont@mairie.fr",
        first_name: "Jean",
        last_name: "Dupont",
    };

    #[test]
    fn mair_757_common_passwords_are_recognised_whatever_their_case() {
        assert!(is_common("password"));
        assert!(is_common("PassWord"));
        assert!(is_common("unbelievable"));
        assert!(!is_common("Velo-Mairie-Riviere-2026"));
    }

    #[test]
    fn mair_757_identity_fragments_are_found_whatever_their_case() {
        assert_eq!(identity_fragment("Jean.Dupont!2026x", JEAN), Some("e-mail"));
        assert_eq!(
            identity_fragment("monprenomJEAN2026", JEAN),
            Some("first name")
        );
        assert_eq!(
            identity_fragment("dupont-mairie-2026", JEAN),
            Some("last name")
        );
        assert_eq!(identity_fragment("Velo-Mairie-Riviere", JEAN), None);
    }

    #[test]
    fn mair_757_short_names_are_not_looked_for() {
        let short = Identity {
            email: "al@mairie.fr",
            first_name: "Al",
            last_name: "Li",
        };
        assert_eq!(identity_fragment("totally-albino-lion", short), None);
    }
}
