//! Migration of the Mairie 360 accounts and roles to Keycloak (MAIR-141).
//!
//! The job reads every account from `v_users_sso_export`, provisions it in the realm through the
//! Admin API and records the link in `user_identities` through `link_user_identity()`. It is
//! built to be replayed without creating duplicates: an account already known to Keycloak (by
//! recorded link, or by e-mail) is updated instead of created, roles are only added when
//! missing, and re-linking the same subject is a no-op in the schema.

use super::admin::{KeycloakAdminClient, KeycloakAdminError, KeycloakRole, KeycloakUserProfile};
use crate::database::admin::sso_export::{ListSsoExportQueryView, SsoExportUser};
use crate::database::auth::link_identity::LinkUserIdentityQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use utoipa::ToSchema;

/// Provider name under which Keycloak identities are stored in `user_identities`.
pub const KEYCLOAK_PROVIDER: &str = "keycloak";

/// Description given to the realm roles the job creates.
const ROLE_DESCRIPTION: &str = "Mairie 360 role, migrated from Core";

/// Knobs of a migration run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationOptions {
    /// Ask Keycloak to e-mail every **newly created, enabled** account a link to set its
    /// password. Accounts that already existed never receive it, so replays send nothing.
    pub send_password_setup_email: bool,
}

/// Failure that stops a migration run.
///
/// Per-account problems (a subject already linked to another account, an e-mail Keycloak
/// refuses) do not stop the run: they are reported in [`UserMigrationResult`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    /// No confidential client, or no Admin API URL can be derived from the realm URL.
    NotConfigured,
    /// Keycloak refused Core's service account or its permissions.
    KeycloakForbidden,
    /// Keycloak could not be reached or answered unexpectedly.
    KeycloakUnavailable,
    /// The export or the link could not be read or written.
    Database,
}

impl Display for MigrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "{}", KeycloakAdminError::NotConfigured),
            Self::KeycloakForbidden => write!(f, "{}", KeycloakAdminError::Forbidden),
            Self::KeycloakUnavailable => write!(f, "{}", KeycloakAdminError::Unavailable),
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<KeycloakAdminError> for MigrationError {
    fn from(error: KeycloakAdminError) -> Self {
        match error {
            KeycloakAdminError::NotConfigured => Self::NotConfigured,
            KeycloakAdminError::Forbidden => Self::KeycloakForbidden,
            KeycloakAdminError::NotFound
            | KeycloakAdminError::AlreadyExists
            | KeycloakAdminError::Unavailable => Self::KeycloakUnavailable,
        }
    }
}

/// What happened to one account during a run.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserMigrationStatus {
    /// The Keycloak account did not exist and was created.
    Created,
    /// The Keycloak account already existed; its profile, roles and link were synchronised.
    Updated,
    /// The account could not be migrated; `error` says why. Replaying after fixing the cause
    /// migrates it.
    Failed,
}

/// Outcome of the migration of one account.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct UserMigrationResult {
    /// Mairie 360 user id.
    #[schema(example = 42)]
    pub user_id: i32,
    /// E-mail of the account, as sent to Keycloak.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    pub email: String,
    /// `created`, `updated` or `failed`.
    pub status: UserMigrationStatus,
    /// Keycloak user id now linked to the account (`user_identities.subject`). `null` only when
    /// the account failed before Keycloak knew it.
    #[schema(nullable, example = "f3b2c1d0-7a6e-4c5b-9d8e-1a2b3c4d5e6f")]
    pub keycloak_id: Option<String>,
    /// `false` when the account is archived: the Keycloak account is disabled.
    #[schema(example = true)]
    pub enabled: bool,
    /// Realm roles mapped during this run (empty on a replay where nothing was missing).
    #[schema(example = json!(["Maire", "User"]))]
    pub roles_added: Vec<String>,
    /// `true` when Keycloak was asked to e-mail a password set-up link (new enabled accounts
    /// only, and only with `send_password_setup_email`).
    #[schema(example = false)]
    pub password_email_sent: bool,
    /// Why the account failed, or why the password e-mail could not be sent; `null` otherwise.
    #[schema(nullable, example = json!(null))]
    pub error: Option<String>,
}

/// Summary of a migration run, followed by the per-account details.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct MigrationReport {
    /// Accounts read from the export (archived ones included).
    #[schema(example = 137)]
    pub total: usize,
    /// Accounts created in Keycloak during this run.
    #[schema(example = 135)]
    pub created: usize,
    /// Accounts that already existed in Keycloak and were synchronised.
    #[schema(example = 2)]
    pub updated: usize,
    /// Accounts that could not be migrated (see each `error`).
    #[schema(example = 0)]
    pub failed: usize,
    /// Accounts provisioned disabled because they are archived in Core.
    #[schema(example = 4)]
    pub disabled: usize,
    /// One entry per account, in id order.
    pub users: Vec<UserMigrationResult>,
}

impl MigrationReport {
    fn push(&mut self, result: UserMigrationResult) {
        self.total += 1;
        match result.status {
            UserMigrationStatus::Created => self.created += 1,
            UserMigrationStatus::Updated => self.updated += 1,
            UserMigrationStatus::Failed => self.failed += 1,
        }
        if !result.enabled && result.status != UserMigrationStatus::Failed {
            self.disabled += 1;
        }
        self.users.push(result);
    }
}

/// Migrates every Mairie 360 account to Keycloak and links it in `user_identities`.
///
/// # Errors
///
/// - [`MigrationError::NotConfigured`] without a confidential client;
/// - [`MigrationError::KeycloakForbidden`] if Keycloak refuses the service account;
/// - [`MigrationError::KeycloakUnavailable`] if Keycloak fails during the run;
/// - [`MigrationError::Database`] if the export cannot be read or a link cannot be written.
///
/// A run stopped by one of these can be replayed: the accounts already processed are then
/// reported as `updated`.
pub async fn migrate_users(
    db: &SmartDatabase,
    admin: &KeycloakAdminClient,
    options: MigrationOptions,
) -> Result<MigrationReport, MigrationError> {
    if !admin.is_configured() {
        return Err(MigrationError::NotConfigured);
    }
    let users: Vec<SsoExportUser> =
        db.fetch_all(&ListSsoExportQueryView::new())
            .await
            .map_err(|e| {
                eprintln!("Keycloak migration: cannot read v_users_sso_export: {e}");
                MigrationError::Database
            })?;

    let mut roles = HashMap::new();
    let mut report = MigrationReport::default();
    for user in &users {
        report.push(migrate_user(db, admin, &mut roles, options, user).await?);
    }
    Ok(report)
}

async fn migrate_user(
    db: &SmartDatabase,
    admin: &KeycloakAdminClient,
    roles: &mut HashMap<String, KeycloakRole>,
    options: MigrationOptions,
    user: &SsoExportUser,
) -> Result<UserMigrationResult, MigrationError> {
    let profile = KeycloakUserProfile {
        email: user.email.clone(),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        enabled: user.enabled,
    };
    let mut result = UserMigrationResult {
        user_id: user.id,
        email: user.email.clone(),
        status: UserMigrationStatus::Failed,
        keycloak_id: None,
        enabled: user.enabled,
        roles_added: Vec::new(),
        password_email_sent: false,
        error: None,
    };

    let Some((keycloak_id, status)) = provision(admin, user, &profile).await? else {
        result.error = Some(
            "Keycloak refuses to create the account: its username or e-mail is already taken by an account with another e-mail."
                .to_string(),
        );
        return Ok(result);
    };
    result.keycloak_id = Some(keycloak_id.clone());
    result.roles_added = sync_roles(admin, roles, &keycloak_id, &user.roles).await?;

    let link = LinkUserIdentityQueryView::new(user.id, KEYCLOAK_PROVIDER, &keycloak_id);
    match db.fetch_scalar::<i32, _>(&link).await {
        Ok(_) => {}
        Err(ApiLibError::Database(DbError::UniqueViolation(_))) => {
            result.error = Some(format!(
                "Keycloak account {keycloak_id} is already linked to another Mairie 360 account."
            ));
            return Ok(result);
        }
        Err(e) => {
            eprintln!("Keycloak migration: cannot link user {}: {e}", user.id);
            return Err(MigrationError::Database);
        }
    }
    result.status = status;

    if status == UserMigrationStatus::Created && options.send_password_setup_email && user.enabled {
        match admin.send_password_setup_email(&keycloak_id).await {
            Ok(()) => result.password_email_sent = true,
            Err(e) => {
                eprintln!(
                    "Keycloak migration: password set-up e-mail not sent to {}: {e}",
                    user.email
                );
                result.error = Some(format!("The password set-up e-mail could not be sent: {e}"));
            }
        }
    }
    Ok(result)
}

/// Finds or creates the Keycloak account of `user` and synchronises its profile. Returns its
/// Keycloak id with `Created` or `Updated`, or `None` when Keycloak refuses the creation because
/// the username or e-mail is taken by an account that cannot be found by e-mail.
async fn provision(
    admin: &KeycloakAdminClient,
    user: &SsoExportUser,
    profile: &KeycloakUserProfile,
) -> Result<Option<(String, UserMigrationStatus)>, MigrationError> {
    // The recorded link wins over the e-mail, which can change on either side; a link to a
    // vanished account (rebuilt realm) falls back to the e-mail.
    let linked = match user.identities.get(KEYCLOAK_PROVIDER) {
        Some(subject) => admin.get_user(subject).await?,
        None => None,
    };
    let existing = match linked {
        Some(user) => Some(user),
        None => admin.find_user_by_email(&user.email).await?,
    };
    if let Some(existing) = existing {
        admin.update_user(&existing.id, profile).await?;
        return Ok(Some((existing.id, UserMigrationStatus::Updated)));
    }

    match admin.create_user(profile).await {
        Ok(id) => Ok(Some((id, UserMigrationStatus::Created))),
        Err(KeycloakAdminError::AlreadyExists) => {
            // Created meanwhile, or known under another spelling of the e-mail: synchronise it.
            let Some(existing) = admin.find_user_by_email(&user.email).await? else {
                return Ok(None);
            };
            admin.update_user(&existing.id, profile).await?;
            Ok(Some((existing.id, UserMigrationStatus::Updated)))
        }
        Err(error) => Err(error.into()),
    }
}

/// Maps the Core roles `names` to Keycloak user `keycloak_id` as realm roles, creating the
/// roles the realm lacks. Roles already mapped, and roles the user holds only in Keycloak
/// (`default-roles-<realm>`, `offline_access`, ...), are left untouched. Returns the names
/// mapped by this call.
async fn sync_roles(
    admin: &KeycloakAdminClient,
    cache: &mut HashMap<String, KeycloakRole>,
    keycloak_id: &str,
    names: &[String],
) -> Result<Vec<String>, MigrationError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let mut wanted = Vec::with_capacity(names.len());
    for name in names {
        if let Some(role) = cache.get(name) {
            wanted.push(role.clone());
        } else {
            let role = ensure_realm_role(admin, name).await?;
            cache.insert(name.clone(), role.clone());
            wanted.push(role);
        }
    }
    let current = admin.user_realm_roles(keycloak_id).await?;
    let missing: Vec<KeycloakRole> = wanted
        .into_iter()
        .filter(|role| !current.iter().any(|mapped| mapped.name == role.name))
        .collect();
    if !missing.is_empty() {
        admin.add_user_realm_roles(keycloak_id, &missing).await?;
    }
    Ok(missing.into_iter().map(|role| role.name).collect())
}

async fn ensure_realm_role(
    admin: &KeycloakAdminClient,
    name: &str,
) -> Result<KeycloakRole, MigrationError> {
    if let Some(role) = admin.realm_role(name).await? {
        return Ok(role);
    }
    admin.create_realm_role(name, ROLE_DESCRIPTION).await?;
    admin.realm_role(name).await?.ok_or_else(|| {
        eprintln!("Keycloak migration: realm role {name} missing right after its creation");
        MigrationError::KeycloakUnavailable
    })
}
