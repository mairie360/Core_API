#![allow(dead_code)]
//! JWTs signed like Core signs them (MAIR-680): `EdDSA` with the lib's test key pair, Core's `kid`,
//! `iss` `mairie360-core` and `aud` `mairie360`. Since `mairie360_api_lib` 6.0.0 there is no HS256
//! any more, so a test that needs a token with chosen claims (expired, extra claims) signs it here.
//! Needs `init_test_jwt_env()` first.

use jsonwebtoken::{decode_header, encode, Algorithm, EncodingKey, Header};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::http_setup::test_jwt_private_key;
use serde_json::Value;

/// Ed25519 signing key from a PKCS#8 PEM (Core builds jsonwebtoken without its `use_pem` feature).
pub fn ed_key(pem: &str) -> EncodingKey {
    use base64::Engine;
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(body)
        .unwrap();
    EncodingKey::from_ed_der(&der)
}

/// The private key Core signs with in this process: `JWT_PRIVATE_KEY` when set (e.g. exported by
/// `stack_secrets.sh`, kept by `init_test_jwt_env()`), else the lib's test key.
pub fn core_private_key() -> String {
    std::env::var("JWT_PRIVATE_KEY")
        .map(|key| key.replace("\\n", "\n"))
        .unwrap_or_else(|_| test_jwt_private_key())
}

/// Signs `claims` (a JSON object) with Core's test key; `iss` and `aud` default to Core's.
pub fn sign_like_core(claims: &Value) -> String {
    let kid = decode_header(&generate_jwt("1", "").unwrap()).unwrap().kid;
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = kid;
    let mut claims = claims.clone();
    let object = claims.as_object_mut().unwrap();
    object
        .entry("iss")
        .or_insert_with(|| "mairie360-core".into());
    object.entry("aud").or_insert_with(|| "mairie360".into());
    encode(&header, &claims, &ed_key(&core_private_key())).unwrap()
}
