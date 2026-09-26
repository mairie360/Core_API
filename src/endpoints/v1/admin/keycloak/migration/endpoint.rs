use super::view::KeycloakMigrationView;
use crate::keycloak::migration::{
    migrate_users, MigrationError, MigrationOptions, MigrationReport,
};
use crate::keycloak::{KeycloakAdminClient, KeycloakClient};
use actix_web::{http::StatusCode, post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeycloakMigrationError {
    DatabaseError,
    KeycloakForbidden,
    KeycloakUnavailable,
    NotConfigured,
}

impl std::fmt::Display for KeycloakMigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => write!(f, "{}", MigrationError::Database),
            Self::KeycloakForbidden => write!(f, "{}", MigrationError::KeycloakForbidden),
            Self::KeycloakUnavailable => write!(f, "{}", MigrationError::KeycloakUnavailable),
            Self::NotConfigured => write!(
                f,
                "Keycloak migration is not configured: Keycloak sign-in must be enabled with a confidential client."
            ),
        }
    }
}

impl ResponseError for KeycloakMigrationError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::KeycloakForbidden | Self::KeycloakUnavailable => StatusCode::BAD_GATEWAY,
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

impl From<MigrationError> for KeycloakMigrationError {
    fn from(error: MigrationError) -> Self {
        match error {
            MigrationError::NotConfigured => Self::NotConfigured,
            MigrationError::KeycloakForbidden => Self::KeycloakForbidden,
            MigrationError::KeycloakUnavailable => Self::KeycloakUnavailable,
            MigrationError::Database => Self::DatabaseError,
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Migrate the accounts and roles to Keycloak",
    description = "Provisions every Mairie 360 account in the Keycloak realm and links it to its \
                   Keycloak user id in `user_identities`, so that nobody has to recreate an \
                   account and no role is lost when the single sign-on is switched on. Reserved \
                   to administrators.\n\n\
                   For each account (archived ones included), in id order: the Keycloak account \
                   is looked up by the link already recorded, then by e-mail, and created when \
                   missing (e-mail as username, e-mail marked verified, **no credential**: users \
                   set their password through Keycloak's own flow, or the \
                   `send_password_setup_email` option); its profile is synchronised; every Core \
                   role of the user is mapped as a realm role of the same name (created in the \
                   realm when missing, never removed from the user); archived accounts are \
                   **disabled** in Keycloak, never deleted; finally the link is recorded, and a \
                   Keycloak user id already linked to another account makes that account `failed` \
                   instead of moving the link.\n\n\
                   **Replayable**: running it again creates no duplicate (existing accounts are \
                   reported `updated`, missing roles are added, links are kept). Accounts created \
                   in Keycloak by hand are adopted by e-mail. A realm rebuilt from scratch is \
                   handled: vanished accounts are recreated and re-linked.\n\n\
                   The run is synchronous, one Admin API round-trip set per account: a few \
                   hundred accounts take seconds, a very large directory should be run from the \
                   `keycloak_migration` binary shipped in the image instead. Core's client must be \
                   confidential with **service accounts enabled** and the `realm-management` \
                   roles `manage-users`, `view-realm` and `manage-realm`. The migration changes \
                   nothing on the local password: `POST /api/v1/auth/login` keeps working.",
    request_body(
        content = Option<KeycloakMigrationView>,
        description = "Run options. Omit the body to use the defaults.",
        example = json!({ "send_password_setup_email": false })
    ),
    responses(
        (
            status = 200,
            description = "Run completed. `failed` counts the accounts that need attention (see their `error`); the run itself succeeded even when it is not 0.",
            body = MigrationReport,
            example = json!({
                "total": 4,
                "created": 2,
                "updated": 1,
                "failed": 1,
                "disabled": 1,
                "users": [
                    {
                        "user_id": 1,
                        "email": "marie.lefevre@mairie360.fr",
                        "status": "created",
                        "keycloak_id": "f3b2c1d0-7a6e-4c5b-9d8e-1a2b3c4d5e6f",
                        "enabled": true,
                        "roles_added": ["Admin"],
                        "password_email_sent": false,
                        "error": null
                    },
                    {
                        "user_id": 2,
                        "email": "jean.dupont@mairie360.fr",
                        "status": "updated",
                        "keycloak_id": "0b7d9e2a-6f14-4c8b-a7d0-1e9f2c4b6a83",
                        "enabled": true,
                        "roles_added": [],
                        "password_email_sent": false,
                        "error": null
                    },
                    {
                        "user_id": 3,
                        "email": "paul.martin@mairie360.fr",
                        "status": "created",
                        "keycloak_id": "5e9a1c3b-2d4f-4a6e-8b7c-9d0e1f2a3b4c",
                        "enabled": false,
                        "roles_added": ["User"],
                        "password_email_sent": false,
                        "error": null
                    },
                    {
                        "user_id": 4,
                        "email": "claire.bernard@mairie360.fr",
                        "status": "failed",
                        "keycloak_id": "0b7d9e2a-6f14-4c8b-a7d0-1e9f2c4b6a83",
                        "enabled": true,
                        "roles_added": [],
                        "password_email_sent": false,
                        "error": "Keycloak account 0b7d9e2a-6f14-4c8b-a7d0-1e9f2c4b6a83 is already linked to another Mairie 360 account."
                    }
                ]
            })
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized: No JWT token provided.")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 500,
            description = "The accounts could not be read from the database, or a link could not be written. Accounts already provisioned are adopted by the next run.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 502,
            description = "Keycloak could not be reached or answered unexpectedly during the run, or refused Core's service account (`KEYCLOAK_CLIENT_SECRET` wrong, service accounts disabled, or missing `realm-management` roles). Replay the migration once fixed.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Unavailable" = (value = json!("Keycloak is unavailable."))),
                ("Service account refused" = (value = json!("Keycloak refused Core's service account.")))
            )
        ),
        (
            status = 503,
            description = "Keycloak sign-in is disabled on this instance (`KEYCLOAK_REALM_URL` or `KEYCLOAK_CLIENT_ID` not set), or the client is public (`KEYCLOAK_CLIENT_SECRET` not set): no service account can call the Admin API.",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak migration is not configured: Keycloak sign-in must be enabled with a confidential client.")
        )
    ),
    tag = "Admin - Keycloak",
    security(
        ("jwt" = [])
    )
)]
#[post("/migration")]
pub async fn run_keycloak_migration(
    payload: Option<web::Json<KeycloakMigrationView>>,
    state: web::Data<AppState>,
    keycloak: Option<web::Data<KeycloakClient>>,
) -> Result<impl Responder, KeycloakMigrationError> {
    let keycloak = keycloak.ok_or(KeycloakMigrationError::NotConfigured)?;
    let admin = KeycloakAdminClient::new(keycloak.config().clone());
    let options = MigrationOptions {
        send_password_setup_email: payload
            .as_ref()
            .is_some_and(|view| view.send_password_setup_email()),
    };

    let report = migrate_users(state.get_smart_db(), &admin, options).await?;
    Ok(HttpResponse::Ok().json(report))
}
