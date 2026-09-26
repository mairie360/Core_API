//! Mirrors the accounts an administrator creates, edits, archives or re-assigns roles to into
//! the Keycloak realm (MAIR-142), so that access is managed from Mairie 360 alone.
//!
//! Every helper here is one step of an administration endpoint. The endpoints call Keycloak
//! **before** writing to Core's database, then compensate the Keycloak side when the database
//! write fails (the account is deleted again, its former profile restored, the role unmapped
//! or re-mapped, ...): a failure on either side answers with an error and leaves Core and
//! Keycloak in the same state as before the call. Keycloak only mirrors the accounts it
//! already knows (by recorded link or by e-mail) plus the ones created through Core: an
//! account not migrated yet (MAIR-141) is left to the migration job.

use super::admin::{KeycloakAdminClient, KeycloakAdminError, KeycloakRole, KeycloakUserProfile};
use super::migration::KEYCLOAK_PROVIDER;
use crate::database::admin::sso_export::{GetSsoExportUserQueryView, SsoExportUser};
use crate::database::auth::link_identity::LinkUserIdentityQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::BuildHasher;

/// Description given to the realm roles Core creates.
pub const ROLE_DESCRIPTION: &str = "Mairie 360 role, managed by Core";

/// Failure of one synchronisation step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// No confidential client, or no Admin API URL can be derived from the realm URL.
    NotConfigured,
    /// Keycloak refused Core's service account or its permissions.
    KeycloakForbidden,
    /// Keycloak could not be reached, or answered unexpectedly.
    KeycloakUnavailable,
    /// Keycloak already has an account with this e-mail (or username) that is not the one being
    /// written.
    EmailTaken,
    /// The Keycloak account is already linked to another Mairie 360 account.
    LinkedToAnotherUser,
    /// The account could not be read from, or the link written to, the database.
    Database,
}

impl Display for SyncError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "{}", KeycloakAdminError::NotConfigured),
            Self::KeycloakForbidden => write!(f, "{}", KeycloakAdminError::Forbidden),
            Self::KeycloakUnavailable => write!(f, "{}", KeycloakAdminError::Unavailable),
            Self::EmailTaken => write!(
                f,
                "Another Keycloak account already uses this e-mail address."
            ),
            Self::LinkedToAnotherUser => write!(
                f,
                "The Keycloak account is already linked to another Mairie 360 account."
            ),
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<KeycloakAdminError> for SyncError {
    fn from(error: KeycloakAdminError) -> Self {
        match error {
            KeycloakAdminError::NotConfigured => Self::NotConfigured,
            KeycloakAdminError::Forbidden => Self::KeycloakForbidden,
            KeycloakAdminError::AlreadyExists => Self::EmailTaken,
            KeycloakAdminError::NotFound | KeycloakAdminError::Unavailable => {
                Self::KeycloakUnavailable
            }
        }
    }
}

/// Keycloak account reserved for a user being created, before the account exists in Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedAccount {
    /// Keycloak user id.
    pub keycloak_id: String,
    /// `true` when this call created the account, `false` when it adopted an account that
    /// already existed with the same e-mail (created by hand in the realm).
    pub created: bool,
}

/// Reads `user_id` as the synchronisation sees it (profile, archived flag, roles, links).
/// `None` for an unknown user.
///
/// # Errors
///
/// [`SyncError::Database`].
pub async fn export_user(
    db: &SmartDatabase,
    user_id: i32,
) -> Result<Option<SsoExportUser>, SyncError> {
    let mut users: Vec<SsoExportUser> = db
        .fetch_all(&GetSsoExportUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("Keycloak sync: cannot read user {user_id} from v_users_sso_export: {e}");
            SyncError::Database
        })?;
    Ok(users.pop())
}

/// The Keycloak profile of an exported account.
#[must_use]
pub fn profile_of(user: &SsoExportUser) -> KeycloakUserProfile {
    KeycloakUserProfile {
        email: user.email.clone(),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        enabled: user.enabled,
    }
}

/// Returns the Keycloak user id of `user`, `None` when Keycloak does not know the user.
///
/// The recorded link is tried first, then (link to a vanished account, or account not
/// migrated yet) the account with the same e-mail, which is linked on the spot. `None` means
/// there is nothing to keep in sync.
///
/// # Errors
///
/// [`SyncError::LinkedToAnotherUser`] if the account found by e-mail is linked to another
/// user, [`SyncError::Database`] if the link cannot be written, plus the Keycloak errors.
pub async fn find_account(
    db: &SmartDatabase,
    admin: &KeycloakAdminClient,
    user: &SsoExportUser,
) -> Result<Option<String>, SyncError> {
    if let Some(subject) = user.identities.get(KEYCLOAK_PROVIDER) {
        if admin.get_user(subject).await?.is_some() {
            return Ok(Some(subject.clone()));
        }
    }
    let Some(existing) = admin.find_user_by_email(&user.email).await? else {
        return Ok(None);
    };
    link_account(db, user.id, &existing.id).await?;
    Ok(Some(existing.id))
}

/// Records that `user_id` is the Keycloak account `keycloak_id`, through the schema's
/// `link_user_identity()`. Re-linking the same pair is a no-op.
///
/// # Errors
///
/// [`SyncError::LinkedToAnotherUser`] if `keycloak_id` is linked to another user,
/// [`SyncError::Database`] otherwise.
pub async fn link_account(
    db: &SmartDatabase,
    user_id: i32,
    keycloak_id: &str,
) -> Result<(), SyncError> {
    let link = LinkUserIdentityQueryView::new(user_id, KEYCLOAK_PROVIDER, keycloak_id);
    match db.fetch_scalar::<i32, _>(&link).await {
        Ok(_) => Ok(()),
        Err(ApiLibError::Database(DbError::UniqueViolation(_))) => {
            eprintln!(
                "Keycloak sync: account {keycloak_id} is already linked to another user than {user_id}"
            );
            Err(SyncError::LinkedToAnotherUser)
        }
        Err(e) => {
            eprintln!("Keycloak sync: cannot link user {user_id} to {keycloak_id}: {e}");
            Err(SyncError::Database)
        }
    }
}

/// Finds or creates the Keycloak account of a user about to be created in Core, and writes
/// `profile` to it. Call [`discard_account`] if the Core account cannot be created after all.
///
/// # Errors
///
/// [`SyncError::EmailTaken`] if Keycloak refuses the e-mail although no account carries it
/// (taken as a username by an account with another e-mail), plus the Keycloak errors.
pub async fn reserve_account(
    admin: &KeycloakAdminClient,
    profile: &KeycloakUserProfile,
) -> Result<ReservedAccount, SyncError> {
    if let Some(existing) = admin.find_user_by_email(&profile.email).await? {
        admin.update_user(&existing.id, profile).await?;
        return Ok(ReservedAccount {
            keycloak_id: existing.id,
            created: false,
        });
    }
    let keycloak_id = admin.create_user(profile).await?;
    Ok(ReservedAccount {
        keycloak_id,
        created: true,
    })
}

/// Undoes [`reserve_account`]: deletes the account it created; an adopted account is kept. A
/// failure is logged, never returned: the caller is already reporting the error that led here.
pub async fn discard_account(admin: &KeycloakAdminClient, account: &ReservedAccount) {
    if !account.created {
        return;
    }
    if let Err(e) = admin.delete_user(&account.keycloak_id).await {
        eprintln!(
            "Keycloak sync: cannot delete the account {} reserved for a user whose creation failed: {e}",
            account.keycloak_id
        );
    }
}

/// Writes `profile` to the Keycloak account `keycloak_id` (names, e-mail, enabled flag).
///
/// # Errors
///
/// [`SyncError::EmailTaken`] if another account carries the new e-mail, plus the Keycloak
/// errors.
pub async fn write_profile(
    admin: &KeycloakAdminClient,
    keycloak_id: &str,
    profile: &KeycloakUserProfile,
) -> Result<(), SyncError> {
    admin.update_user(keycloak_id, profile).await?;
    Ok(())
}

/// Disables the Keycloak account `keycloak_id` and ends its sessions: the user cannot sign in
/// any more and is signed out of the clients that honour the back-channel logout.
///
/// # Errors
///
/// The Keycloak errors.
pub async fn disable_account(
    admin: &KeycloakAdminClient,
    keycloak_id: &str,
) -> Result<(), SyncError> {
    admin.set_enabled(keycloak_id, false).await?;
    admin.logout_user(keycloak_id).await?;
    Ok(())
}

/// Re-enables the Keycloak account `keycloak_id` (undoes [`disable_account`], sessions
/// excepted).
///
/// # Errors
///
/// The Keycloak errors.
pub async fn enable_account(
    admin: &KeycloakAdminClient,
    keycloak_id: &str,
) -> Result<(), SyncError> {
    admin.set_enabled(keycloak_id, true).await?;
    Ok(())
}

/// Asks Keycloak to e-mail the account `keycloak_id` a link to set its password.
///
/// # Errors
///
/// The Keycloak errors ([`SyncError::KeycloakUnavailable`] also when the realm cannot send
/// e-mails).
pub async fn invite(admin: &KeycloakAdminClient, keycloak_id: &str) -> Result<(), SyncError> {
    admin.send_password_setup_email(keycloak_id).await?;
    Ok(())
}

/// Maps the realm role `name` to the account `keycloak_id`, creating the role when missing.
///
/// Returns `false` when the account already held it, so the caller knows whether to unmap it
/// when compensating.
///
/// # Errors
///
/// The Keycloak errors.
pub async fn map_role(
    admin: &KeycloakAdminClient,
    keycloak_id: &str,
    name: &str,
) -> Result<bool, SyncError> {
    let role = ensure_realm_role(admin, name).await?;
    let current = admin.user_realm_roles(keycloak_id).await?;
    if current.iter().any(|mapped| mapped.name == role.name) {
        return Ok(false);
    }
    admin
        .add_user_realm_roles(keycloak_id, std::slice::from_ref(&role))
        .await?;
    Ok(true)
}

/// Unmaps the realm role `name` from the account `keycloak_id`.
///
/// Returns `false` when the account did not hold it (or the realm has no such role), so the
/// caller knows whether to re-map it when compensating.
///
/// # Errors
///
/// The Keycloak errors.
pub async fn unmap_role(
    admin: &KeycloakAdminClient,
    keycloak_id: &str,
    name: &str,
) -> Result<bool, SyncError> {
    let Some(role) = admin.realm_role(name).await? else {
        return Ok(false);
    };
    let current = admin.user_realm_roles(keycloak_id).await?;
    if !current.iter().any(|mapped| mapped.name == role.name) {
        return Ok(false);
    }
    admin
        .remove_user_realm_roles(keycloak_id, std::slice::from_ref(&role))
        .await?;
    Ok(true)
}

/// Maps the Core roles `names` to Keycloak user `keycloak_id` as realm roles.
///
/// Roles the realm lacks are created. Roles already mapped, and roles the user holds only in
/// Keycloak (`default-roles-<realm>`, `offline_access`, ...), are left untouched. `cache`
/// spares one look-up per role across calls. Returns the names mapped by this call.
///
/// # Errors
///
/// The Keycloak errors.
pub async fn sync_roles<S: BuildHasher>(
    admin: &KeycloakAdminClient,
    cache: &mut HashMap<String, KeycloakRole, S>,
    keycloak_id: &str,
    names: &[String],
) -> Result<Vec<String>, SyncError> {
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

/// Returns the realm role `name`, creating it when the realm lacks it.
///
/// # Errors
///
/// The Keycloak errors.
pub async fn ensure_realm_role(
    admin: &KeycloakAdminClient,
    name: &str,
) -> Result<KeycloakRole, SyncError> {
    if let Some(role) = admin.realm_role(name).await? {
        return Ok(role);
    }
    admin.create_realm_role(name, ROLE_DESCRIPTION).await?;
    admin.realm_role(name).await?.ok_or_else(|| {
        eprintln!("Keycloak sync: realm role {name} missing right after its creation");
        SyncError::KeycloakUnavailable
    })
}
