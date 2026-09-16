use crate::database::roles::does_role_exist::DoesRoleExistQueryView;
use crate::database::roles::patch_role::PatchRoleQueryView;
use crate::endpoints::v1::admin::roles::patch::view::PatchView;

use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchError {
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            PatchError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for PatchError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            PatchError::NotFound => StatusCode::NOT_FOUND,
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

async fn patch_role(
    id: u64,
    payload: PatchView,
    state: web::Data<AppState>,
) -> Result<(), PatchError> {
    let smart_db = state.get_smart_db();
    if !does_role_exist(id, smart_db).await {
        return Err(PatchError::NotFound);
    }
    let view = PatchRoleQueryView::new(
        id,
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );
    if !view.is_noop() {
        smart_db
            .execute(view)
            .await
            .map_err(|_| PatchError::DatabaseError)?;
    }
    Ok(())
}

#[utoipa::path(
    patch,
    path = "/{id}",
    summary = "Modifier un rôle",
    description = "Met à jour partiellement un rôle : seuls les champs présents dans le corps sont \
                   écrits, les autres restent inchangés. Réservé aux administrateurs.\n\n\
                   Un corps vide est accepté et ne déclenche aucune écriture, mais l'existence du \
                   rôle est vérifiée au préalable : un identifiant inconnu répond `404` même sans \
                   rien à modifier.\n\n\
                   `can_be_deleted` est doublement optionnel : l'omettre laisse la valeur \
                   actuelle, alors que `null` l'efface. La réponse a un corps vide.",
    request_body(
        content = PatchView,
        description = "Champs à modifier. Tous facultatifs.",
        example = json!({ "description": "Agent municipal habilité à instruire les dossiers" })
    ),
    responses(
        (
            status = 200,
            description = "Rôle mis à jour, ou rien à mettre à jour. Corps vide.",
        ),
        (
            status = 400,
            description = "Corps JSON malformé, ou `id` du chemin qui n'est pas un entier.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: invalid type: integer `1`, expected a string")
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
            description = "Aucun rôle ne porte cet identifiant.",
            body = String,
            content_type = "text/plain",
            example = json!("The requested resource was not found.")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la mise à jour, par exemple un nom déjà pris par un autre rôle.",
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
#[patch("/{id}")]
pub async fn admin_patch_role(
    id: web::Path<u64>,
    payload: web::Json<PatchView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PatchError> {
    patch_role(id.into_inner(), payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
