use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use utoipa::ToSchema;

/// Every account with what the Keycloak migration job needs to provision it, read from the
/// schema's `v_users_sso_export` view (MAIR-141), ordered by id.
///
/// Archived users are included so the job can disable (never delete) their Keycloak account.
#[derive(serde::Deserialize)]
pub struct ListSsoExportQueryView {
    params: Vec<QueryParam>,
}

impl ListSsoExportQueryView {
    #[must_use]
    pub const fn new() -> Self {
        Self { params: Vec::new() }
    }
}

impl Default for ListSsoExportQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for ListSsoExportQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM ( \
            SELECT id, email, first_name, last_name, enabled, has_local_password, roles, identities \
            FROM v_users_sso_export \
            ORDER BY id \
        ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ListSsoExportQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListSsoExportQueryView")
    }
}

/// One account as the Keycloak migration job sees it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct SsoExportUser {
    /// Mairie 360 user id.
    #[schema(example = 42)]
    pub id: i32,
    /// E-mail address, used to find or create the Keycloak account.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    pub email: String,
    /// First name, copied to the Keycloak profile.
    #[schema(example = "Jean")]
    pub first_name: String,
    /// Last name, copied to the Keycloak profile.
    #[schema(example = "Dupont")]
    pub last_name: String,
    /// `false` when the account is archived: it is provisioned disabled in Keycloak and cannot
    /// sign in.
    #[schema(example = true)]
    pub enabled: bool,
    /// `true` while the account can still sign in with `POST /api/v1/auth/login`; `false` once it
    /// is SSO-only (no local password).
    #[schema(example = true)]
    pub has_local_password: bool,
    /// Names of the Core roles held by the user, sorted; empty for a user without any role.
    #[schema(example = json!(["Maire", "User"]))]
    pub roles: Vec<String>,
    /// Provider links already recorded (provider name to subject). `{}` for a user not yet
    /// migrated; `keycloak` holds the Keycloak user id once they are.
    #[schema(example = json!({ "keycloak": "f3b2c1d0-7a6e-4c5b-9d8e-1a2b3c4d5e6f" }))]
    pub identities: BTreeMap<String, String>,
}
