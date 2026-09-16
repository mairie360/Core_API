use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

use crate::database::users::delete_user::DeleteUserQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserError {
    AlreadyDeleted,
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserError::AlreadyDeleted => write!(f, "User is already deleted"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserError::AlreadyDeleted => StatusCode::OK,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user(state: web::Data<AppState>, user_id: u64) -> Result<(), DeleteUserError> {
    let view = DeleteUserQueryView::new(user_id);
    state.get_smart_db().execute(view).await.map_err(|e| {
        eprintln!("Error: {}", e);
        DeleteUserError::AlreadyDeleted
    })?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    summary = "Supprimer un utilisateur (administration)",
    description = "Supprime un compte utilisateur. Réservé aux administrateurs.\n\n\
                   Attention à la lecture du statut : une suppression réussie répond `204` avec un \
                   corps vide, tandis qu'un **échec** — compte déjà supprimé, identifiant inconnu, \
                   ou panne de base — répond `200` avec un message en texte brut. Un client ne \
                   peut donc pas se contenter de tester `2xx` : il doit distinguer `204` de `200`.\n\n\
                   Ce endpoint ne renvoie jamais `500`.",
    params(
        ("userId" = u64, Path, description = "Identifiant de l'utilisateur.", example = 42)
    ),
    responses(
        (
            status = 204,
            description = "Compte supprimé. Corps vide.",
        ),
        (
            status = 200,
            description = "La suppression n'a rien fait : compte déjà supprimé, identifiant inconnu, ou échec de l'écriture.",
            body = String,
            content_type = "text/plain",
            example = json!("User is already deleted")
        ),
        (
            status = 400,
            description = "L'`userId` du chemin n'est pas un entier.",
            body = String,
            content_type = "text/plain",
            example = json!("can not parse \"abc\" to a u64")
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
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn admin_delete_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, DeleteUserError> {
    delete_user(state, path.into_inner()).await?;

    Ok(HttpResponse::NoContent())
}
