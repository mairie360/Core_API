// Les dépendances transitives (via mairie360_api_lib et le reste de l'arbre) imposent plusieurs
// versions de certains crates (base64, bitflags, ...) ; on ne maîtrise pas ces versions depuis ce
// repo, donc ce lint clippy::cargo est désactivé plutôt que laissé en échec permanent.
#![allow(clippy::multiple_crate_versions)]
pub mod client_ip;
pub mod database;
pub mod emails;
pub mod endpoints;
pub mod keycloak;
pub mod passwords;
pub mod phone;
pub mod rate_limit;
pub mod redis_keys;
pub mod refresh_token;
pub mod session_jwt;
pub mod session_revocation;
pub mod telemetry;
