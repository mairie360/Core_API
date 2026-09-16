use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_user_from_group::DeleteUserFromGroupQueryView;
use crate::database::groups::is_user_member::IsUserMemberQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserFromGroupError {
    BadRequest,
    UnknowUser,
}

impl std::fmt::Display for DeleteUserFromGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserFromGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
            DeleteUserFromGroupError::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for DeleteUserFromGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserFromGroupError::BadRequest => StatusCode::BAD_REQUEST,
            DeleteUserFromGroupError::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user_from_group(
    state: web::Data<AppState>,
    group_id: u64,
    user_id: u64,
) -> Result<(), DeleteUserFromGroupError> {
    let smart_db = state.get_smart_db();

    let user_check_view = IsUserMemberQueryView::new(group_id, user_id);
    let result: bool = smart_db
        .fetch_scalar(&user_check_view)
        .await
        .map_err(|_| DeleteUserFromGroupError::UnknowUser)?;
    if !result {
        return Err(DeleteUserFromGroupError::UnknowUser);
    }

    let db_view = DeleteUserFromGroupQueryView::new(group_id, user_id);
    smart_db
        .execute(db_view)
        .await
        .map_err(|_| DeleteUserFromGroupError::BadRequest)?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "/",
    summary = "Retirer un utilisateur d'un groupe",
    description = "Détache un utilisateur d'un groupe. Le compte utilisateur et le groupe sont \
                   conservés ; seul le lien d'appartenance disparaît.\n\n\
                   Contrairement à la suppression d'un groupe, cet appel n'est pas idempotent : \
                   retirer un utilisateur qui n'est pas membre répond `404`.\n\n\
                   Cet endpoint ne vérifie pas que l'appelant est propriétaire du groupe.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe.", example = 3),
        ("user_id" = u64, Path, description = "Identifiant de l'utilisateur à retirer.", example = 42)
    ),
    responses(
        (
            status = 204,
            description = "Utilisateur retiré du groupe. Corps vide.",
        ),
        (
            status = 400,
            description = "Échec de la suppression du lien une fois l'appartenance confirmée. Ce endpoint renvoie `400` là où les autres renverraient `500`.",
            body = String,
            content_type = "text/plain",
            example = json!("Bad request.")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "L'utilisateur n'est pas membre de ce groupe, ou l'un des deux identifiants n'existe pas.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow user.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn remove_user_from_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(u64, u64)>,
) -> Result<impl Responder, DeleteUserFromGroupError> {
    let (group_id, user_id) = path.into_inner();
    delete_user_from_group(state, group_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
