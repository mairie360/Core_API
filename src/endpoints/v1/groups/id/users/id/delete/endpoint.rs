use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_user_from_group::DeleteUserFromGroupQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserFromGroupError {
    BadRequest,
    UnknowUser,
}

impl std::fmt::Display for DeleteUserFromGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => {
                write!(f, "Bad request.")
            }
            Self::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for DeleteUserFromGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::UnknowUser => StatusCode::NOT_FOUND,
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
    let removed: i64 = state
        .get_smart_db()
        .fetch_scalar(&DeleteUserFromGroupQueryView::new(group_id, user_id))
        .await
        .map_err(|e| {
            eprintln!("Remove user from group DB Error: {e}");
            DeleteUserFromGroupError::BadRequest
        })?;
    if removed == 0 {
        return Err(DeleteUserFromGroupError::UnknowUser);
    }
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
                   Requires the `update` right on the group (owner, `update_all`, or an ACL): \
                   any other caller gets `403` and the membership is kept.",
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
            status = 403,
            description = "The caller has no `update` right on the group (owner, `update_all` such as administrators, or an ACL), checked by `access_guard_middleware` before the handler runs.",
            body = String,
            content_type = "text/plain",
            example = json!("Insufficient permissions")
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
