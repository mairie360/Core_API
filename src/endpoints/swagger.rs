use crate::endpoints::health::HealthDoc;
use crate::endpoints::hello::HelloDoc;
use crate::endpoints::v1::doc::V1Doc;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Core API — Mairie 360",
        version = "1.0.0",
        description = "\
Central API of the **Mairie 360** platform. It holds the users, sessions, roles, groups and \
resource access rights: the other APIs (Project, Calendar, Message, ELearning) rely on it to \
authenticate and authorize their calls.

## Authentication

Every route under `/api` is protected by `JwtMiddleware`. The token is obtained from \
`POST /api/v1/auth/login`, which returns it in the `Authorization` response header \
(`Bearer <jwt>`) together with a refresh token in the body. Send it on every call in the \
`Authorization` header, and renew it with `POST /api/v1/sessions/refresh`, which also **rotates** the refresh token: keep the one it \
returns, the previous one stops working.

Routes under `/api/v1/admin` also require the user to be an administrator, otherwise they \
answer `403 Forbidden`.

The public authentication routes (login, keycloak, forgot/reset/force_change_password, refresh) \
are rate limited per client address (and per e-mail address where one is sent): beyond the \
budget they answer `429 Too Many Requests` with a `Retry-After` header.

## Error format

Error responses (`4xx` and `5xx`) have a **`text/plain`** body holding the error message, not a \
JSON object. The only exceptions are documented explicitly per operation (for example the `412` \
of `POST /api/v1/auth/login`, which returns a JSON object). Every response carries \
`X-Content-Type-Options: nosniff`.

Statuses returned across the API, before the handler runs:

| Status | Meaning |
| --- | --- |
| `400` | Malformed body or query string, or a field breaking its validation rules (length, \
format, control characters, `<` / `>` in names and descriptions); the body names the first \
invalid field, e.g. ``Invalid `email`: must be a valid e-mail address``. |
| `401` | `Authorization` header missing or malformed, invalid or expired JWT, or revoked session. |
| `403` | Valid JWT but insufficient rights (`admin` route or resource access control). |
| `500` | Database, Redis or external service failure. |
",
        contact(
            name = "Équipe Mairie 360",
            url = "https://github.com/mairie360"
        ),
        license(
            name = "Propriétaire",
            identifier = "LicenseRef-mairie360-proprietary"
        )
    ),
    servers(
        (url = "http://localhost:3000", description = "Développement local (cargo run)"),
        (url = "http://development.mairie360.fr", description = "Pile Docker de développement (nginx)")
    ),
    tags(
        (name = "Auth", description = "Login and password lifecycle (forgotten password, reset, forced change at first connection). Accounts are created by administrators only (`POST /api/v1/admin/users/`)."),
        (name = "Sessions", description = "Sessions de l'utilisateur connecté : liste des sessions actives, historique, rafraîchissement et révocation."),
        (name = "Users", description = "Annuaire des utilisateurs et profil de l'utilisateur connecté."),
        (name = "Roles", description = "Consultation des rôles disponibles sur la plateforme."),
        (name = "Groups", description = "Groupes d'utilisateurs et gestion de leurs membres."),
        (name = "Admin - Users", description = "Administration des comptes utilisateurs. Réservé aux administrateurs."),
        (name = "Admin - Roles", description = "Administration des rôles et de leurs permissions. Réservé aux administrateurs."),
        (name = "Admin - Keycloak", description = "Migration of the accounts and roles to the Keycloak realm (single sign-on). Reserved to administrators."),
        (name = "Service", description = "Sondes techniques non authentifiées, utilisées par Docker et Kubernetes.")
    ),
    nest(
        (path = "/api/v1", api = V1Doc),
        (path = "/", api = HealthDoc),
        (path = "/", api = HelloDoc),
    ),
    modifiers(&SecurityAddon) // On ajoute le modifier ici
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "jwt",
            SecurityScheme::Http(
                Http::builder()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some(
                        "JWT obtenu via `POST /api/v1/auth/login` (en-tête de réponse \
                         `Authorization`) puis renouvelé via `POST /api/v1/sessions/refresh`.",
                    ))
                    .build(),
            ),
        );
    }
}
