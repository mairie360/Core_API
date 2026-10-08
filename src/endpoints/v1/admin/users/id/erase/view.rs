use crate::database::users::erasure::AnonymizeUserResult;
use serde::Serialize;
use utoipa::ToSchema;

/// What the erasure did (MAIR-289).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ErasureView {
    /// The erased account (the row stays, without identity, so that shared objects keep their
    /// references).
    #[schema(example = 42)]
    pub user_id: i32,
    /// `true` when the account had already been anonymized: nothing was changed.
    #[schema(example = false)]
    pub already_anonymized: bool,
    /// The administrator who took over the user's groups, projects and non-private events.
    #[schema(example = 2, nullable)]
    pub handed_over_to: Option<i32>,
    /// Private events of the user, deleted.
    #[schema(example = 3, nullable)]
    pub deleted_private_events: Option<i64>,
    /// Sessions of the user, closed and published to the revocation list.
    #[schema(example = 2)]
    pub revoked_sessions: usize,
    /// Whether the Keycloak account of the user was deleted (`false` when Keycloak is not
    /// configured or does not know the account).
    #[schema(example = false)]
    pub keycloak_account_deleted: bool,
}

impl ErasureView {
    #[must_use]
    pub const fn new(result: &AnonymizeUserResult, keycloak_account_deleted: bool) -> Self {
        Self {
            user_id: result.user_id,
            already_anonymized: result.already_anonymized,
            handed_over_to: result.handed_over_to,
            deleted_private_events: result.deleted_private_events,
            revoked_sessions: result.revoked_sessions.len(),
            keycloak_account_deleted,
        }
    }
}
