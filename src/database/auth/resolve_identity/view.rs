use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Maps the identity asserted by an external provider back to the local account, through the
/// schema's `resolve_user_identity()` (MAIR-141).
///
/// `provider` names the identity provider (`keycloak` for the platform SSO) and `subject` is the
/// stable identifier it issues for the account (the `sub` claim of its tokens), never the e-mail.
/// The function returns `NULL` when the identity is unknown **or** when the linked account is
/// archived: archived users keep their link but cannot sign in.
#[derive(serde::Deserialize)]
pub struct ResolveUserIdentityQueryView {
    provider: String,
    subject: String,
    params: Vec<QueryParam>,
}

impl ResolveUserIdentityQueryView {
    #[must_use]
    pub fn new(provider: &str, subject: &str) -> Self {
        Self {
            provider: provider.to_string(),
            subject: subject.to_string(),
            params: vec![
                QueryParam::Text(provider.to_string()),
                QueryParam::Text(subject.to_string()),
            ],
        }
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

impl ApiRequestDto for ResolveUserIdentityQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT resolve_user_identity($1, $2) AS user_id) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ResolveUserIdentityQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResolveUserIdentityQueryView: provider = {}, subject = {}",
            self.provider, self.subject
        )
    }
}

/// Result of [`ResolveUserIdentityQueryView`]: the active account linked to the identity, if any.
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ResolveUserIdentityQueryResultView {
    user_id: Option<i32>,
}

impl ResolveUserIdentityQueryResultView {
    #[must_use]
    pub const fn new(user_id: Option<i32>) -> Self {
        Self { user_id }
    }

    /// `None` when no account is linked to the identity or the linked account is archived.
    #[must_use]
    pub const fn user_id(&self) -> Option<i32> {
        self.user_id
    }
}
