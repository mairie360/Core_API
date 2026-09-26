use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Records that a user is known to an identity provider under a given subject, through the
/// schema's `link_user_identity()` (MAIR-141), the single write path of `user_identities`.
///
/// Built to be replayed safely by the Keycloak migration job and by the SSO login:
/// - a user already linked to the provider with the same subject is left untouched;
/// - a user already linked to the provider with another subject is re-linked (the provider
///   re-issued the account, e.g. a rebuilt realm);
/// - a subject already linked to **another** user is refused with a unique violation
///   (`DbError::UniqueViolation`): an identity is never silently moved between accounts;
/// - an unknown user is refused with a foreign-key violation (`DbError::ForeignKeyViolation`).
///
/// Archived users can be linked (the job provisions them disabled in Keycloak); the login path
/// refuses them separately. Fetch it as a scalar: it returns the `user_identities.id` of the link.
#[derive(serde::Deserialize)]
pub struct LinkUserIdentityQueryView {
    user_id: i32,
    provider: String,
    subject: String,
    params: Vec<QueryParam>,
}

impl LinkUserIdentityQueryView {
    #[must_use]
    pub fn new(user_id: i32, provider: &str, subject: &str) -> Self {
        Self {
            user_id,
            provider: provider.to_string(),
            subject: subject.to_string(),
            params: vec![
                QueryParam::I32(user_id),
                QueryParam::Text(provider.to_string()),
                QueryParam::Text(subject.to_string()),
            ],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }

    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }
}

impl ApiRequestDto for LinkUserIdentityQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT link_user_identity($1, $2, $3)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for LinkUserIdentityQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LinkUserIdentityQueryView: user_id = {}, provider = {}, subject = {}",
            self.user_id, self.provider, self.subject
        )
    }
}
