use crate::database::roles::delete_role::DeleteRoleQueryView;
use crate::database::roles::lock_role::LockRoleQueryView;
use crate::endpoints::admin_guard::AdminUser;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum DeleteError {
    Forbidden,
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::NotFound => {
                write!(f, "The requested resource was not found.")
            }
            Self::Forbidden => {
                write!(f, "The requested resource cannot be deleted.")
            }
        }
    }
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Checks and deletes the role in one transaction, the role locked from the first read
/// (MAIR-420): its `can_be_deleted` flag cannot change between the check and the deletion.
async fn delete_role(id: u64, state: web::Data<AppState>) -> Result<(), DeleteError> {
    let database_error = |e: ApiLibError| {
        tracing::error!("Delete role DB Error: {e}");
        DeleteError::DatabaseError
    };
    let mut tx = state.get_smart_db().begin().await.map_err(database_error)?;
    let can_be_deleted: Vec<bool> = tx
        .fetch_all(&LockRoleQueryView::new(id))
        .await
        .map_err(database_error)?;
    match can_be_deleted.first() {
        None => return Err(DeleteError::NotFound),
        Some(false) => return Err(DeleteError::Forbidden),
        Some(true) => {}
    }
    tx.execute(&DeleteRoleQueryView::new(id))
        .await
        .map_err(database_error)?;
    tx.commit().await.map_err(database_error)
}

#[utoipa::path(
    delete,
    path = "/{id}",
    summary = "Supprimer un rôle",
    description = "Supprime un rôle. Réservé aux administrateurs.\n\n\
                   Certains rôles sont protégés par leur drapeau `can_be_deleted` : leur \
                   suppression est refusée en `403`. Ce `403` a donc deux causes possibles sur ce \
                   endpoint : appelant non administrateur, ou rôle non supprimable — le message du \
                   corps permet de les distinguer.\n\n\
                   Contrairement à la suppression d'un groupe, l'appel n'est pas idempotent : un \
                   identifiant inconnu répond `404`.",
    responses(
        (
            status = 204,
            description = "Rôle supprimé. Corps vide.",
        ),
        (
            status = 400,
            description = "L'`id` du chemin n'est pas un entier.",
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
            description = "L'appelant n'est pas administrateur, ou le rôle est marqué non supprimable.",
            body = String,
            content_type = "text/plain",
            example = json!("The requested resource cannot be deleted.")
        ),
        (
            status = 404,
            description = "Aucun rôle ne porte cet identifiant.",
            body = String,
            content_type = "text/plain",
            example = json!("The requested resource was not found.")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la suppression.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    params(
        ("id" = u64, Path, description = "Role id. The base roles (1 to 5: Admin, Maire, Responsable, User, Guest) are protected; the roles created afterwards start at 6.", example = 6)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[delete("/{id}")]
pub async fn admin_delete_role(
    _: AdminUser,
    id: web::Path<u64>,
    state: web::Data<AppState>,
) -> Result<impl Responder, DeleteError> {
    delete_role(id.into_inner(), state).await?;
    Ok(HttpResponse::NoContent())
}
