use crate::endpoints::v1::admin::keycloak::migration::endpoint;
use crate::keycloak::migration::{MigrationReport, UserMigrationResult, UserMigrationStatus};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(endpoint::run_keycloak_migration),
    components(schemas(
        super::view::KeycloakMigrationView,
        MigrationReport,
        UserMigrationResult,
        UserMigrationStatus
    ))
)]
pub struct MigrationDoc;
