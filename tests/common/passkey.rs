//! Software authenticator of the passkey tests (MAIR-505): a `SoftPasskey` of
//! `webauthn-authenticator-rs` answers the ceremonies like a browser would, without hardware or
//! browser.

use sqlx::PgPool;
use url::Url;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_rs::prelude::{
    Base64UrlSafeData, CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse,
};
use webauthn_rs_proto::AllowCredentials;

/// Relying party of the test apps: the sign-in page of `mairie360.test`.
pub const RP_ID: &str = "mairie360.test";
pub const ORIGIN: &str = "https://login.mairie360.test";

/// One authenticator, holding the passkeys it created.
pub struct Authenticator {
    inner: WebauthnAuthenticator<SoftPasskey>,
    /// Credential ids of the passkeys created, newest last.
    pub credential_ids: Vec<Vec<u8>>,
}

impl Default for Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl Authenticator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // The soft passkey cannot verify a user: it claims it did (the server requires it).
            inner: WebauthnAuthenticator::new(SoftPasskey::new(true)),
            credential_ids: Vec::new(),
        }
    }

    /// Answers a registration ceremony for `origin`.
    pub fn register(
        &mut self,
        origin: &str,
        options: CreationChallengeResponse,
    ) -> RegisterPublicKeyCredential {
        let credential = self
            .inner
            .do_registration(Url::parse(origin).unwrap(), options)
            .expect("the soft passkey registers");
        self.credential_ids.push(credential.raw_id.to_vec());
        credential
    }

    /// Answers a sign-in ceremony for `origin` with the passkey `credential_id`.
    ///
    /// Core sends a discoverable request (no `allowCredentials`), which the soft passkey does
    /// not support: the credential is named here, as a browser lets the user pick one. The
    /// assertion is the same either way.
    pub fn authenticate(
        &mut self,
        origin: &str,
        mut options: RequestChallengeResponse,
        credential_id: &[u8],
    ) -> PublicKeyCredential {
        options.public_key.allow_credentials = vec![AllowCredentials {
            type_: "public-key".to_string(),
            id: Base64UrlSafeData::from(credential_id),
            transports: None,
        }];
        self.inner
            .do_authentication(Url::parse(origin).unwrap(), options)
            .expect("the soft passkey signs")
    }

    /// The credential id of the last passkey created.
    #[must_use]
    pub fn last_credential_id(&self) -> &[u8] {
        self.credential_ids.last().expect("a passkey was created")
    }
}

/// DDL of `releases/v3.1.0/03__user_passkeys.sql` (Database, MAIR-505). Until `TEST_DB_VERSION`
/// (`.cargo/config.toml`) points at an image that carries the table, the tests create it
/// themselves; every statement is idempotent, so this is a no-op on a newer image. Remove once
/// the version is bumped.
const USER_PASSKEYS_DDL: [&str; 2] = [
    "CREATE TABLE IF NOT EXISTS user_passkeys (
        id SERIAL PRIMARY KEY,
        user_id INT NOT NULL,
        credential_id BYTEA NOT NULL,
        passkey JSONB NOT NULL,
        label VARCHAR(100) NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
        last_used_at TIMESTAMPTZ,
        CONSTRAINT fk_user_passkeys_user FOREIGN KEY (user_id)
            REFERENCES users(id) ON DELETE CASCADE,
        CONSTRAINT uq_user_passkeys_credential_id UNIQUE (credential_id),
        CONSTRAINT chk_user_passkeys_credential_id
            CHECK (octet_length(credential_id) BETWEEN 16 AND 1023),
        CONSTRAINT chk_user_passkeys_passkey CHECK (jsonb_typeof(passkey) = 'object'),
        CONSTRAINT chk_user_passkeys_label CHECK (btrim(label) <> '')
    )",
    "CREATE INDEX IF NOT EXISTS idx_user_passkeys_user_id ON user_passkeys (user_id)",
];

/// Makes sure the shared test database has `user_passkeys` (see [`USER_PASSKEYS_DDL`]).
pub async fn ensure_user_passkeys_table(raw: &PgPool) {
    for statement in USER_PASSKEYS_DDL {
        sqlx::query(statement)
            .execute(raw)
            .await
            .expect("user_passkeys table of Database v3.1.0");
    }
}
