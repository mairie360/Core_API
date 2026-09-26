use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Options of a Keycloak migration run. The body is optional: without it, every option takes
/// its default.
#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct KeycloakMigrationView {
    /// Ask Keycloak to e-mail a password set-up link (`UPDATE_PASSWORD` action) to every
    /// account **created** by this run and not archived. Accounts that already existed in
    /// Keycloak never receive it, so replaying the migration sends nothing. Requires the
    /// realm's SMTP settings; a failed e-mail is reported per account and does not fail the
    /// migration. Defaults to `false`.
    #[schema(example = false)]
    #[serde(default)]
    send_password_setup_email: bool,
}

impl KeycloakMigrationView {
    #[must_use]
    pub const fn send_password_setup_email(&self) -> bool {
        self.send_password_setup_email
    }
}
