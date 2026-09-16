use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::users::patch_user::PatchUserQueryView;
use crate::endpoints::v1::user::me::patch::view::PatchMeView;

#[derive(Debug, Clone, PartialEq)]
enum PatchMeError {
    DatabaseError,
}

impl std::fmt::Display for PatchMeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchMeError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for PatchMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchMeError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_patch_me(
    state: web::Data<AppState>,
    view: PatchMeView,
    user_id: u64,
) -> Result<(), PatchMeError> {
    let db_view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        view.phone(),
        None,
    );
    if !db_view.is_noop() {
        state.get_smart_db().execute(db_view).await.map_err(|e| {
            eprintln!("Error: {:?}", e);
            PatchMeError::DatabaseError
        })?;
    }
    Ok(())
}

#[utoipa::path(
    patch,
    path = "/",
    summary = "Modifier son propre profil",
    description = "Met à jour l'état civil, l'adresse e-mail ou le téléphone de l'utilisateur \
                   porté par le JWT. Modification partielle : seuls les champs présents dans le \
                   corps sont écrits, les autres restent inchangés.\n\n\
                   Un corps vide, ou ne contenant que des `null`, est accepté et ne déclenche \
                   aucune écriture : la réponse reste `200`.\n\n\
                   Le mot de passe et les rôles ne se modifient pas ici : passer par \
                   `/api/v1/auth/forgot_password` pour le mot de passe et par \
                   `/api/v1/admin/users/` pour les rôles. La réponse a un corps vide ; il faut \
                   rappeler `GET /api/v1/user/me/` pour relire le profil.",
    request_body(
        content = PatchMeView,
        description = "Champs à modifier. Tous facultatifs ; un champ absent ou `null` est ignoré.",
        example = json!({
            "first_name": "Jean",
            "phone": "0798765432"
        })
    ),
    responses(
        (
            status = 200,
            description = "Profil mis à jour, ou rien à mettre à jour. Corps vide.",
        ),
        (
            status = 400,
            description = "Corps JSON malformé, ou champ d'un type inattendu.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: invalid type: integer `42`, expected a string")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la mise à jour du profil.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn patch_me(
    state: web::Data<AppState>,
    view: web::Json<PatchMeView>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PatchMeError> {
    trigger_patch_me(state, view.into_inner(), auth_user.id).await?;
    Ok(HttpResponse::Ok())
}
