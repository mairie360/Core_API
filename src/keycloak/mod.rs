//! Single sign-on through Keycloak (OIDC authorization code flow).
//!
//! Core redeems the authorization code obtained by the front, verifies the returned ID token and
//! matches its verified e-mail to an existing Mairie 360 account. The session it then opens is
//! the same as after a password login (Core JWT + refresh token), so fronts, BFFs and the other
//! APIs keep working unchanged and the user keeps the roles stored in Core.

mod client;
mod config;
mod error;

pub use client::{AuthorizationCode, KeycloakClient, KeycloakIdentity};
pub use config::KeycloakConfig;
pub use error::KeycloakError;
