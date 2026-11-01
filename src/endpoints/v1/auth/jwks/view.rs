use serde::Serialize;
use utoipa::ToSchema;

/// One Ed25519 public key Core signs session JWTs with (RFC 8037 OKP JWK).
#[derive(Debug, Serialize, ToSchema)]
pub struct JwkView {
    /// Key type, always `OKP` (octet key pair).
    #[schema(example = "OKP")]
    pub kty: String,
    /// Curve, always `Ed25519`.
    #[schema(example = "Ed25519")]
    pub crv: String,
    /// The 32-byte public key, base64url without padding.
    #[schema(example = "KW8bV4kiFUo8hmOEU3-lXdqZL1wmKvBw6smUyjpmXks")]
    pub x: String,
    /// RFC 7638 thumbprint of the key: the `kid` header of the JWTs it verifies.
    #[schema(example = "636czspyshqVvkoHmLoy2qx0PD-QuW3R2a1XnN5peL0")]
    pub kid: String,
    /// Signature algorithm, always `EdDSA`.
    #[schema(example = "EdDSA")]
    pub alg: String,
    /// Key use, always `sig`.
    #[schema(rename = "use", example = "sig")]
    #[serde(rename = "use")]
    pub use_: String,
}

/// The public keys session JWTs are verified with (`JWT_PUBLIC_KEY`), as a JSON Web Key Set.
#[derive(Debug, Serialize, ToSchema)]
pub struct JwksView {
    /// Every published key: the current one and, during a rotation, the next or previous one.
    /// Empty while the instance still signs with the historical shared secret.
    pub keys: Vec<JwkView>,
}
