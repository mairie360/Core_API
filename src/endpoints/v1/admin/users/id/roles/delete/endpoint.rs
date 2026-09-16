use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

use crate::database::users::remove_role::RemoveRolesQueryView;

#[derive(Debug, Clone, PartialEq)]
enum RemoveUserRoleError {
    NotFound,
}

impl std::fmt::Display for RemoveUserRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for RemoveUserRoleError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user(
    state: web::Data<AppState>,
    user_id: u64,
    role_id: u64,
) -> Result<(), RemoveUserRoleError> {
    let view = RemoveRolesQueryView::new(role_id, user_id);
    state
        .get_smart_db()
        .execute(view)
        .await
        .map_err(|_| RemoveUserRoleError::NotFound)?;

    Ok(())
}

#[utoipa::path(
    delete,
    summary = "Retirer un rôle à un utilisateur",
    description = "Détache un rôle d'un compte utilisateur. Le rôle lui-même et le compte sont \
                   conservés ; seule l'attribution disparaît. Réservé aux administrateurs.\n\n\
                   Contrairement à l'attribution, les deux identifiants sont bien lus dans le \
                   chemin. Tout échec est rapporté en `404` : ce endpoint ne renvoie jamais `500`.",
    params(
        ("userId" = u64, Path, description = "Identifiant de l'utilisateur.", example = 42),
        ("roleId" = u64, Path, description = "Identifiant du rôle à retirer.", example = 2)
    ),
    path = "/{roleId}",
    responses(
        (
            status = 204,
            description = "Rôle retiré. Corps vide.",
        ),
        (
            status = 400,
            description = "`userId` ou `roleId` n'est pas un entier.",
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
        (
            status = 404,
            description = "L'utilisateur ne porte pas ce rôle, ou l'un des deux identifiants n'existe pas.",
            body = String,
            content_type = "text/plain",
            example = json!("The requested resource was not found.")
        ),
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Users"
)]
#[delete("/{roleId}")]
pub async fn admin_delete_user_role(
    state: web::Data<AppState>,
    params: web::Path<(u64, u64)>,
) -> Result<impl Responder, RemoveUserRoleError> {
    let (user_id, role_id) = params.into_inner();
    delete_user(state, user_id, role_id).await?;
    Ok(HttpResponse::NoContent())
}
