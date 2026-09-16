use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_group::DeleteGroupQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteGroupError {
    BadRequest,
}

impl std::fmt::Display for DeleteGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for DeleteGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_delete_group(state: web::Data<AppState>, id: u64) -> Result<(), DeleteGroupError> {
    let db_view = DeleteGroupQueryView::new(id);
    state
        .get_smart_db()
        .execute(db_view)
        .await
        .map_err(|_| DeleteGroupError::BadRequest)?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    summary = "Supprimer un groupe",
    description = "Supprime un groupe et les liens d'appartenance de ses membres. Les comptes \
                   utilisateurs eux-mêmes ne sont pas touchés.\n\n\
                   Opération idempotente : supprimer un groupe déjà supprimé ou inexistant répond \
                   également `204`, sans erreur.\n\n\
                   Attention : cet endpoint ne vérifie pas que l'appelant est propriétaire du \
                   groupe. Tout utilisateur authentifié peut supprimer n'importe quel groupe.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe.", example = 3)
    ),
    responses(
        (
            status = 204,
            description = "Groupe supprimé, ou déjà absent. Corps vide.",
        ),
        (
            status = 400,
            description = "Échec de la suppression en base. Ce endpoint renvoie `400` là où les autres renverraient `500`.",
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
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn delete_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
) -> Result<impl Responder, DeleteGroupError> {
    trigger_delete_group(state, id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
