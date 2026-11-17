//! Passkeys (`WebAuthn` / FIDO2) served by Core (MAIR-810), on top of `webauthn-rs`.
//!
//! `WEBAUTHN_RP_ID` (the registrable domain the passkeys are bound to, e.g. `mairie360-eip.fr`) and
//! `WEBAUTHN_RP_ORIGIN` (the origin of the login front, e.g. `https://login.mairie360-eip.fr`) switch
//! the feature on: without them [`Passkeys::from_env`] is `None` and every passkey route answers
//! `503`. The ceremony states (the challenge and what the server must remember about it) live in
//! Redis for two minutes and are read **once**: a replayed challenge finds nothing.

use mairie360_api_lib::env_manager::get_env_var;
use mairie360_api_lib::redis::redis_interface::Redis;
use uuid::Uuid;
use webauthn_rs::prelude::{DiscoverableAuthentication, PasskeyRegistration, Url};
use webauthn_rs::{Webauthn, WebauthnBuilder};

/// Relying party identifier (registrable domain).
pub const WEBAUTHN_RP_ID: &str = "WEBAUTHN_RP_ID";
/// Origin of the front that calls `navigator.credentials` (scheme, host and port, no path).
pub const WEBAUTHN_RP_ORIGIN: &str = "WEBAUTHN_RP_ORIGIN";

/// Time to live of a ceremony state: the challenge the browser signs.
pub const CEREMONY_TTL_SECONDS: u64 = 120;

/// Fixed namespace of the user handles: the same account always gets the same handle.
const USER_HANDLE_NAMESPACE: Uuid = Uuid::from_u128(0x6d61_6972_6965_3336_3070_6173_736b_6579);

/// The `WebAuthn` relying party of the instance, registered as app data by `main`.
pub struct Passkeys {
    webauthn: Webauthn,
}

impl Passkeys {
    /// `rp_origin` is one origin, or several separated by commas (the fronts that register and use
    /// passkeys live on different hosts: `https://login.<domain>,https://settings.<domain>`). The
    /// first is the primary one; each must be an origin of `rp_id`.
    ///
    /// # Errors
    ///
    /// A message when an origin is not a URL or `rp_id` is not a domain of it.
    pub fn new(rp_id: &str, rp_origin: &str) -> Result<Self, String> {
        let origins = rp_origin
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(|origin| {
                Url::parse(origin).map_err(|e| format!("{WEBAUTHN_RP_ORIGIN} is not a URL: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (primary, others) = origins
            .split_first()
            .ok_or_else(|| format!("{WEBAUTHN_RP_ORIGIN} holds no origin"))?;
        let builder = WebauthnBuilder::new(rp_id, primary)
            .map_err(|e| format!("{WEBAUTHN_RP_ID} / {WEBAUTHN_RP_ORIGIN}: {e}"))?;
        let webauthn = others
            .iter()
            .fold(builder, WebauthnBuilder::append_allowed_origin)
            .rp_name("Mairie 360")
            .build()
            .map_err(|e| format!("WebAuthn configuration: {e}"))?;
        Ok(Self { webauthn })
    }

    /// Reads [`WEBAUTHN_RP_ID`] and [`WEBAUTHN_RP_ORIGIN`]: `None` when neither is set (the feature
    /// is off).
    ///
    /// # Errors
    ///
    /// A message when only one of them is set, or when they do not fit together: a half
    /// configuration must stop the start, not silently turn the passkeys off.
    pub fn from_env() -> Result<Option<Self>, String> {
        match (get_env_var(WEBAUTHN_RP_ID), get_env_var(WEBAUTHN_RP_ORIGIN)) {
            (None, None) => Ok(None),
            (Some(id), Some(origin)) => Self::new(id.trim(), origin.trim()).map(Some),
            _ => Err(format!(
                "{WEBAUTHN_RP_ID} and {WEBAUTHN_RP_ORIGIN} must be set together"
            )),
        }
    }

    #[must_use]
    pub const fn webauthn(&self) -> &Webauthn {
        &self.webauthn
    }
}

/// The user handle an authenticator stores with a discoverable credential: stable per account,
/// carries no personal data.
#[must_use]
pub fn user_handle(user_id: u64) -> Uuid {
    Uuid::new_v5(&USER_HANDLE_NAMESPACE, user_id.to_string().as_bytes())
}

fn login_key(id: &str) -> String {
    format!("passkey_login/{id}")
}

fn register_key(user_id: u64) -> String {
    format!("passkey_register/{user_id}")
}

async fn store<T: serde::Serialize + Sync>(
    redis: &Redis,
    key: &str,
    state: &T,
) -> Result<(), String> {
    let value = serde_json::to_string(state).map_err(|e| e.to_string())?;
    // `set_ex` overwrites: a new ceremony replaces an unfinished one.
    redis
        .set_ex(key, value, CEREMONY_TTL_SECONDS)
        .await
        .map_err(|e| e.to_string())
}

async fn take<T: serde::de::DeserializeOwned>(redis: &Redis, key: &str) -> Option<T> {
    let raw = redis.secure_get::<String>(key).await.ok()??;
    // Read once: gone before the answer is checked, so a failed attempt burns the challenge.
    let _ = redis.secure_delete(key).await;
    serde_json::from_str(&raw).ok()
}

/// Remembers the state of a sign-in ceremony under a fresh id and returns that id.
///
/// # Errors
///
/// A message for the logs when Redis refuses the write.
pub async fn begin_login(
    redis: &Redis,
    state: &DiscoverableAuthentication,
) -> Result<String, String> {
    let id = crate::refresh_token::generate();
    store(redis, &login_key(&id), state).await?;
    Ok(id)
}

/// Takes the state of a sign-in ceremony: `None` when the id is unknown, expired or already used.
pub async fn take_login(redis: &Redis, id: &str) -> Option<DiscoverableAuthentication> {
    take(redis, &login_key(id)).await
}

/// Remembers the registration ceremony of `user_id` (one at a time).
///
/// # Errors
///
/// A message for the logs when Redis refuses the write.
pub async fn begin_register(
    redis: &Redis,
    user_id: u64,
    state: &PasskeyRegistration,
) -> Result<(), String> {
    store(redis, &register_key(user_id), state).await
}

/// Takes the registration ceremony of `user_id`: `None` when none is pending or it is spent.
pub async fn take_register(redis: &Redis, user_id: u64) -> Option<PasskeyRegistration> {
    take(redis, &register_key(user_id)).await
}
