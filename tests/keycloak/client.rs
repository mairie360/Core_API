use crate::common::keycloak_mock::{
    id_token_claims, sign, subject_for, KeycloakMock, TestKey, CLIENT_ID, CLIENT_SECRET,
    REDIRECT_URI,
};
use core_api::keycloak::{
    AuthorizationCode, KeycloakClient, KeycloakConfig, KeycloakError, KeycloakIdentity,
};
use jsonwebtoken::{encode, get_current_timestamp, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};

const EMAIL: &str = "claire.martin@mairie360.test";

const fn code(nonce: Option<&'static str>) -> AuthorizationCode<'static> {
    AuthorizationCode {
        code: "auth-code-123",
        redirect_uri: REDIRECT_URI,
        code_verifier: Some("pkce-verifier-456"),
        nonce,
    }
}

fn valid_token(mock: &KeycloakMock) -> String {
    sign(
        &id_token_claims(mock.realm_url(), EMAIL),
        TestKey::A,
        TestKey::A.kid(),
    )
}

fn token_with(mock: &KeycloakMock, change: impl FnOnce(&mut Value)) -> String {
    let mut claims = id_token_claims(mock.realm_url(), EMAIL);
    change(&mut claims);
    sign(&claims, TestKey::A, TestKey::A.kid())
}

#[tokio::test]
async fn test_authenticate_returns_verified_identity() {
    let mock = KeycloakMock::start();
    mock.respond_with_id_token(&valid_token(&mock));

    let identity = mock.client().authenticate(&code(Some("test-nonce"))).await;

    assert_eq!(
        identity,
        Ok(KeycloakIdentity {
            subject: subject_for(EMAIL),
            email: EMAIL.to_string(),
        })
    );
}

#[tokio::test]
async fn test_authenticate_sends_code_client_credentials_and_pkce_verifier() {
    let mock = KeycloakMock::start();
    mock.respond_with_id_token(&valid_token(&mock));

    mock.client().authenticate(&code(None)).await.unwrap();

    let requests = mock.token_requests();
    assert_eq!(requests.len(), 1);
    let form = &requests[0];
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["code"], "auth-code-123");
    assert_eq!(form["redirect_uri"], REDIRECT_URI);
    assert_eq!(form["client_id"], CLIENT_ID);
    assert_eq!(form["client_secret"], CLIENT_SECRET);
    assert_eq!(form["code_verifier"], "pkce-verifier-456");
}

#[tokio::test]
async fn test_public_client_sends_no_secret_nor_verifier_when_absent() {
    let mock = KeycloakMock::start();
    mock.respond_with_id_token(&valid_token(&mock));
    let client = KeycloakClient::new(KeycloakConfig::new(mock.realm_url(), None, CLIENT_ID, None));

    client
        .authenticate(&AuthorizationCode {
            code: "auth-code-123",
            redirect_uri: REDIRECT_URI,
            code_verifier: None,
            nonce: None,
        })
        .await
        .unwrap();

    let form = &mock.token_requests()[0];
    assert!(!form.contains_key("client_secret"));
    assert!(!form.contains_key("code_verifier"));
}

#[tokio::test]
async fn test_rejected_code_is_invalid_grant() {
    let mock = KeycloakMock::start();
    mock.respond_with(
        400,
        json!({ "error": "invalid_grant", "error_description": "Code not valid" }),
    );

    let result = mock.client().authenticate(&code(None)).await;

    assert_eq!(result, Err(KeycloakError::InvalidGrant));
}

#[tokio::test]
async fn test_rejected_client_credentials_is_unavailable() {
    let mock = KeycloakMock::start();
    mock.respond_with(
        401,
        json!({ "error": "unauthorized_client", "error_description": "Invalid client secret" }),
    );

    let result = mock.client().authenticate(&code(None)).await;

    assert_eq!(result, Err(KeycloakError::Unavailable));
}

#[tokio::test]
async fn test_token_endpoint_server_error_is_unavailable() {
    let mock = KeycloakMock::start();
    mock.respond_with(503, json!({}));

    let result = mock.client().authenticate(&code(None)).await;

    assert_eq!(result, Err(KeycloakError::Unavailable));
}

#[tokio::test]
async fn test_unreachable_keycloak_is_unavailable() {
    // Port 9 (discard) is closed on test machines: the connection is refused immediately.
    let client = KeycloakClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        None,
        CLIENT_ID,
        None,
    ));

    let result = client.authenticate(&code(None)).await;

    assert_eq!(result, Err(KeycloakError::Unavailable));
}

#[tokio::test]
async fn test_token_response_without_id_token_is_invalid() {
    let mock = KeycloakMock::start();
    // What Keycloak returns when the authorization request lacked the `openid` scope.
    mock.respond_with(
        200,
        json!({ "access_token": "opaque", "token_type": "Bearer", "expires_in": 300 }),
    );

    let result = mock.client().authenticate(&code(None)).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_unreadable_token_response_is_unavailable() {
    let mock = KeycloakMock::start();
    mock.respond_with(200, json!("not an object"));

    let result = mock.client().authenticate(&code(None)).await;

    assert_eq!(result, Err(KeycloakError::Unavailable));
}

#[tokio::test]
async fn test_verify_rejects_token_signed_by_unknown_key() {
    let mock = KeycloakMock::start();
    // Signed by key B while claiming to be key A: the signature check must fail.
    let forged = sign(
        &id_token_claims(mock.realm_url(), EMAIL),
        TestKey::B,
        TestKey::A.kid(),
    );

    let result = mock.client().verify_id_token(&forged, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_rejects_unknown_kid_after_refreshing_keys() {
    let mock = KeycloakMock::start();
    let token = sign(
        &id_token_claims(mock.realm_url(), EMAIL),
        TestKey::B,
        TestKey::B.kid(),
    );

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
    assert_eq!(mock.jwks_hits(), 1);
}

#[tokio::test]
async fn test_verify_picks_up_rotated_keys() {
    let mock = KeycloakMock::start();
    let client = mock.client();
    client
        .verify_id_token(&valid_token(&mock), None)
        .await
        .unwrap();

    // Keycloak rotates to key B: the unknown kid triggers exactly one new download.
    mock.publish_keys(&[TestKey::A, TestKey::B]);
    let rotated = sign(
        &id_token_claims(mock.realm_url(), EMAIL),
        TestKey::B,
        TestKey::B.kid(),
    );
    let result = client.verify_id_token(&rotated, None).await;

    assert!(result.is_ok(), "{result:?}");
    assert_eq!(mock.jwks_hits(), 2);
}

#[tokio::test]
async fn test_verify_caches_the_key_set() {
    let mock = KeycloakMock::start();
    let client = mock.client();

    for _ in 0..3 {
        client
            .verify_id_token(&valid_token(&mock), None)
            .await
            .unwrap();
    }

    assert_eq!(mock.jwks_hits(), 1);
}

#[tokio::test]
async fn test_verify_rejects_wrong_issuer() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| {
        claims["iss"] = json!("http://127.0.0.1:1/realms/other");
    });

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_accepts_configured_public_issuer() {
    let mock = KeycloakMock::start();
    // Inside a cluster Core reaches Keycloak through an internal URL, while tokens carry the
    // public hostname.
    let public_issuer = "https://auth.mairie360.test/realms/test";
    let client = KeycloakClient::new(KeycloakConfig::new(
        mock.realm_url(),
        Some(public_issuer),
        CLIENT_ID,
        Some(CLIENT_SECRET),
    ));
    let token = token_with(&mock, |claims| claims["iss"] = json!(public_issuer));

    let result = client.verify_id_token(&token, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn test_verify_rejects_token_for_another_client() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| claims["aud"] = json!("another-client"));

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_accepts_audience_list_containing_client() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| {
        claims["aud"] = json!(["account", CLIENT_ID]);
    });

    let result = mock.client().verify_id_token(&token, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn test_verify_rejects_expired_token() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| {
        claims["exp"] = json!(get_current_timestamp() - 120);
    });

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_tolerates_small_clock_skew() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| {
        claims["exp"] = json!(get_current_timestamp() - 5);
    });

    let result = mock.client().verify_id_token(&token, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn test_verify_rejects_nonce_mismatch() {
    let mock = KeycloakMock::start();

    let result = mock
        .client()
        .verify_id_token(&valid_token(&mock), Some("another-nonce"))
        .await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_rejects_missing_nonce_when_expected() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| {
        claims.as_object_mut().unwrap().remove("nonce");
    });

    let result = mock
        .client()
        .verify_id_token(&token, Some("test-nonce"))
        .await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_rejects_unverified_email() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| claims["email_verified"] = json!(false));

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::EmailNotVerified));
}

#[tokio::test]
async fn test_verify_rejects_missing_email() {
    let mock = KeycloakMock::start();
    let token = token_with(&mock, |claims| {
        let claims = claims.as_object_mut().unwrap();
        claims.remove("email");
        claims.remove("email_verified");
    });

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::EmailNotVerified));
}

#[tokio::test]
async fn test_verify_rejects_hmac_token() {
    let mock = KeycloakMock::start();
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(TestKey::A.kid().to_string());
    let token = encode(
        &header,
        &id_token_claims(mock.realm_url(), EMAIL),
        &EncodingKey::from_secret(b"guessable"),
    )
    .unwrap();

    let result = mock.client().verify_id_token(&token, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
    // Refused on its header alone, before any key lookup.
    assert_eq!(mock.jwks_hits(), 0);
}

#[tokio::test]
async fn test_verify_rejects_token_without_kid() {
    let mock = KeycloakMock::start();
    let valid = valid_token(&mock);
    let (_header, rest) = valid.split_once('.').unwrap();
    // `{"alg":"RS256","typ":"JWT"}`, base64url-encoded: same token, header without `kid`.
    let without_kid = format!("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.{rest}");

    let result = mock.client().verify_id_token(&without_kid, None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_verify_rejects_garbage() {
    let mock = KeycloakMock::start();

    let result = mock.client().verify_id_token("not.a.jwt", None).await;

    assert_eq!(result, Err(KeycloakError::InvalidIdToken));
}

#[tokio::test]
async fn test_unreachable_key_set_is_unavailable() {
    let mock = KeycloakMock::start();
    let client = KeycloakClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        Some(mock.realm_url()),
        CLIENT_ID,
        None,
    ));

    let result = client.verify_id_token(&valid_token(&mock), None).await;

    assert_eq!(result, Err(KeycloakError::Unavailable));
}

#[test]
fn test_error_messages() {
    assert_eq!(
        KeycloakError::InvalidGrant.to_string(),
        "Keycloak rejected the authorization code."
    );
    assert_eq!(
        KeycloakError::InvalidIdToken.to_string(),
        "Invalid Keycloak ID token."
    );
    assert_eq!(
        KeycloakError::EmailNotVerified.to_string(),
        "The Keycloak account has no verified e-mail address."
    );
    assert_eq!(
        KeycloakError::Unavailable.to_string(),
        "Keycloak is unavailable."
    );
}
