use crate::database::roles::create_role::CreateRoleQueryView;
use crate::endpoints::v1::admin::roles::view::RoleWriteView;

use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostError {
    Duplicate,
}

impl std::fmt::Display for PostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicate => {
                write!(f, "A role with this name already exists.")
            }
        }
    }
}

impl ResponseError for PostError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Duplicate => StatusCode::CONFLICT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn create_role(payload: RoleWriteView, state: web::Data<AppState>) -> Result<(), PostError> {
    let view = CreateRoleQueryView::new(
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );

    state
        .get_smart_db()
        .execute(view)
        .await
        .map_err(|_| PostError::Duplicate)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/",
    summary = "Créer un rôle",
    description = "Crée un rôle, identifié par son nom qui doit être unique. Réservé aux \
                   administrateurs.\n\n\
                   La réponse a un corps vide et ne renvoie pas l'identifiant attribué : relire la \
                   liste avec `GET /api/v1/admin/roles/` pour le récupérer.\n\n\
                   Tout échec d'écriture en base est rapporté en `409`, y compris une panne sans \
                   rapport avec un doublon : ce endpoint ne renvoie jamais `500`.",
    request_body(
        content = RoleWriteView,
        description = "Nom, description et caractère supprimable du rôle.",
        example = json!({
            "name": "agent",
            "description": "Agent municipal",
            "can_be_deleted": true
        })
    ),
    responses(
        (
            status = 200,
            description = "Rôle créé. Corps vide.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, or a field breaking its rules: `name` 1 to 64 characters, not blank, no control character, no `<` or `>`; `description` at most 1000 characters, no `<` or `>`, no control character other than line breaks and tabs.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `name`: must not contain `<` or `>`")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "L'utilisateur est authentifié mais n'est pas administrateur.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 409,
            description = "Un rôle porte déjà ce nom — ou, plus largement, l'insertion en base a échoué.",
            body = String,
            content_type = "text/plain",
            example = json!("A role with this name already exists.")
        ),
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[post("/")]
pub async fn admin_post_role(
    payload: ValidatedJson<RoleWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PostError> {
    create_role(payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
