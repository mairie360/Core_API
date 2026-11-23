//! Queries of the passkeys (MAIR-810): `user_passkeys`.
//!
//! The credential identifier (`BYTEA`) travels as base64 text (`decode` / `encode` in SQL) and the
//! serialized credential (`JSONB`) as text cast in SQL: the query parameters have no byte-array nor
//! JSON variant. A passkey is only ever read by its owner or, at sign-in, by its credential id.

use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

/// Row of [`ListPasskeysQueryView`] and [`InsertPasskeyQueryView`]: what an agent sees of a passkey
/// (never the key itself).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct PasskeyRecord {
    pub id: i32,
    pub label: String,
    /// Name of the passkey manager deduced from the AAGUID, `None` when unknown (or registered before
    /// the name was kept).
    pub provider: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Row of [`ListPasskeySecretsQueryView`]: the serialized credential of one of the user's passkeys.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PasskeySecretRecord {
    pub passkey: serde_json::Value,
}

/// Row of [`FindPasskeyQueryView`]: the passkey a sign-in names and its (active) owner.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PasskeyOwnerRecord {
    pub id: i32,
    pub user_id: i32,
    pub passkey: serde_json::Value,
}

/// Row of [`DeletePasskeyQueryView`].
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct DeletedPasskeyRecord {
    pub label: String,
}

/// Row of [`UpdatePasskeyQueryView`].
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct UpdatedPasskeyRecord {
    pub id: i32,
}

macro_rules! passkey_view {
    ($(#[$doc:meta])* $name:ident ($($arg:ident: $ty:ty),*) -> $params:expr, $sql:expr) => {
        $(#[$doc])*
        #[derive(serde::Deserialize)]
        pub struct $name {
            params: Vec<QueryParam>,
        }

        impl $name {
            #[must_use]
            pub fn new($($arg: $ty),*) -> Self {
                Self { params: $params }
            }
        }

        impl ApiRequestDto for $name {
            fn query_sql(&self) -> &'static str {
                $sql
            }

            fn query_params(&self) -> &[QueryParam] {
                &self.params
            }
        }
    };
}

passkey_view!(
    /// The passkeys of a user, oldest first.
    ListPasskeysQueryView(user_id: u64)
        -> vec![QueryParam::I32(id_to_sql(user_id))],
    "SELECT row_to_json(t) FROM (\
        SELECT id, label, provider, created_at, last_used_at FROM user_passkeys \
        WHERE user_id = $1 ORDER BY created_at, id\
    ) t"
);

passkey_view!(
    /// The serialized credentials of a user, to exclude them from a new registration.
    ListPasskeySecretsQueryView(user_id: u64)
        -> vec![QueryParam::I32(id_to_sql(user_id))],
    "SELECT row_to_json(t) FROM (SELECT passkey FROM user_passkeys WHERE user_id = $1 ORDER BY id) t"
);

passkey_view!(
    /// Stores a passkey and returns it. A credential id already registered (by anyone) is a unique
    /// violation: `409`.
    InsertPasskeyQueryView(user_id: u64, credential_id_base64: &str, passkey_json: &str, label: &str, provider: Option<&str>)
        -> vec![
            QueryParam::I32(id_to_sql(user_id)),
            QueryParam::Text(credential_id_base64.to_string()),
            QueryParam::Text(passkey_json.to_string()),
            QueryParam::Text(label.to_string()),
            // No NULL parameter exists: an empty text stands for "unknown" (see `NULLIF` below).
            QueryParam::Text(provider.unwrap_or_default().to_string()),
        ],
    "WITH i AS (\
        INSERT INTO user_passkeys (user_id, credential_id, passkey, label, provider) \
        VALUES ($1, decode($2, 'base64'), $3::jsonb, $4, NULLIF($5, '')) \
        RETURNING id, label, provider, created_at, last_used_at\
    ) SELECT row_to_json(i) FROM i"
);

passkey_view!(
    /// Removes a passkey of the user; `DbError::NotFound` when it is not theirs (or does not exist).
    DeletePasskeyQueryView(user_id: u64, passkey_id: u64)
        -> vec![QueryParam::I32(id_to_sql(user_id)), QueryParam::I32(id_to_sql(passkey_id))],
    "WITH d AS (DELETE FROM user_passkeys WHERE user_id = $1 AND id = $2 RETURNING label) \
     SELECT row_to_json(d) FROM d"
);

passkey_view!(
    /// The passkey a sign-in names, if its owner is neither archived nor suspended (MAIR-904); `DbError::NotFound` otherwise.
    FindPasskeyQueryView(credential_id_base64: &str)
        -> vec![QueryParam::Text(credential_id_base64.to_string())],
    "SELECT row_to_json(t) FROM (\
        SELECT p.id, p.user_id, p.passkey FROM user_passkeys p \
        JOIN users u ON u.id = p.user_id \
        WHERE p.credential_id = decode($1, 'base64') AND u.is_archived IS NOT TRUE \
          AND u.status IS DISTINCT FROM 'suspended'\
    ) t"
);

passkey_view!(
    /// Saves the credential after a sign-in (signature counter, backup state) and its date.
    UpdatePasskeyQueryView(passkey_id: i32, passkey_json: &str)
        -> vec![QueryParam::I32(passkey_id), QueryParam::Text(passkey_json.to_string())],
    "WITH u AS (\
        UPDATE user_passkeys SET passkey = $2::jsonb, last_used_at = now() WHERE id = $1 RETURNING id\
    ) SELECT row_to_json(u) FROM u"
);

/// Row of [`ActiveAccountQueryView`].
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ActiveAccountRecord {
    pub id: i32,
}

passkey_view!(
    /// The account, if it is neither archived nor suspended (MAIR-904); `DbError::NotFound` otherwise.
    ActiveAccountQueryView(user_id: u64)
        -> vec![QueryParam::I32(id_to_sql(user_id))],
    "SELECT row_to_json(t) FROM (\
        SELECT u.id FROM users u WHERE u.id = $1 AND u.is_archived IS NOT TRUE \
          AND u.status IS DISTINCT FROM 'suspended'\
    ) t"
);

/// Row of [`UserContactQueryView`]: where the security e-mail goes.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct UserContactRecord {
    pub email: String,
    pub first_name: String,
}

passkey_view!(
    /// E-mail address and first name of a user, for the security e-mails.
    UserContactQueryView(user_id: u64)
        -> vec![QueryParam::I32(id_to_sql(user_id))],
    "SELECT row_to_json(t) FROM (SELECT email, first_name FROM users WHERE id = $1) t"
);
