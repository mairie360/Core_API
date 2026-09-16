use crate::database::roles::can_delete_role::CanDeleteRoleQueryView;
use crate::database::roles::delete_role::DeleteRoleQueryView;
use crate::database::roles::does_role_exist::DoesRoleExistQueryView;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
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

async fn does_role_exist(id: u64, smart_db: &SmartDatabase) -> bool {
    let view = DoesRoleExistQueryView::new(id);
    smart_db.fetch_scalar(&view).await.unwrap()
}

async fn can_delete_role(id: u64, smart_db: &SmartDatabase) -> bool {
    let view = CanDeleteRoleQueryView::new(id);
    smart_db.fetch_scalar(&view).await.unwrap()
}

async fn delete_role(id: u64, state: web::Data<AppState>) -> Result<(), DeleteError> {
    let smart_db = state.get_smart_db();
    if !does_role_exist(id, smart_db).await {
        return Err(DeleteError::NotFound);
    }
    if !can_delete_role(id, smart_db).await {
        return Err(DeleteError::Forbidden);
    }
    let view = DeleteRoleQueryView::new(id);
    smart_db
        .execute(view)
        .await
        .map_err(|_| DeleteError::DatabaseError)
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
        ("id" = u64, Path, description = "Identifiant du rôle.", example = 2)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[delete("/{id}")]
pub async fn admin_delete_role(
    id: web::Path<u64>,
    state: web::Data<AppState>,
) -> Result<impl Responder, DeleteError> {
    delete_role(id.into_inner(), state).await?;
    Ok(HttpResponse::NoContent())
}
