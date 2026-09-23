//! Single sign-on through Keycloak (OIDC authorization code flow) and migration of the
//! existing accounts to the realm.
//!
//! Core redeems the authorization code obtained by the front, verifies the returned ID token and
//! maps it to a Mairie 360 account: first by the identity recorded in `user_identities`
//! (provider `keycloak`, subject = Keycloak user id), then, for an account not linked yet, by
//! its verified e-mail, which links it on the spot. The session it then opens is the same as
//! after a password login (Core JWT + refresh token), so fronts, BFFs and the other APIs keep
//! working unchanged and the user keeps the roles stored in Core.
//!
//! [`migration`] provisions every existing account and its roles in the realm through the
//! Admin API and records the links, so nobody has to recreate an account by hand
//! (MAIR-141). [`sync`] then keeps the realm in step with what administrators change in
//! Core: accounts created, edited or archived, roles granted or revoked (MAIR-142).

mod admin;
mod client;
mod config;
mod error;
pub mod migration;
pub mod sync;

pub use admin::{
    KeycloakAdminClient, KeycloakAdminError, KeycloakRole, KeycloakUser, KeycloakUserProfile,
};
pub use client::{AuthorizationCode, KeycloakClient, KeycloakIdentity};
pub use config::KeycloakConfig;
pub use error::KeycloakError;
