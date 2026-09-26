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
API centrale de la plateforme **Mairie 360**. Elle détient les utilisateurs, les sessions, \
les rôles, les groupes et les droits d'accès aux ressources : les autres APIs (Project, Calendar, \
Message, ELearning) s'appuient sur elle pour authentifier et autoriser leurs appels.

## Authentification

Toutes les routes sous `/api` sont protégées par `JwtMiddleware`. Le jeton s'obtient via \
`POST /api/v1/auth/login`, qui le renvoie dans l'en-tête de réponse `Authorization` \
(`Bearer <jwt>`) accompagné d'un jeton de rafraîchissement dans le corps. Il faut ensuite le \
présenter sur chaque appel dans l'en-tête `Authorization`, et le renouveler via \
`POST /api/v1/sessions/refresh` avant son expiration (`JWT_TIMEOUT`).

Les routes sous `/api/v1/admin` exigent en plus que l'utilisateur soit administrateur \
(`AdminMiddleware`), sans quoi elles répondent `403 Forbidden`.

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
        (name = "Auth", description = "Connexion, inscription et cycle de vie du mot de passe (oubli, réinitialisation, changement forcé à la première connexion)."),
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
