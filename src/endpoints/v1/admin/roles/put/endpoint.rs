use crate::database::roles::change_role::ChangeRoleQueryView;
use crate::database::roles::does_role_exist::DoesRoleExistQueryView;
use crate::endpoints::v1::admin::roles::view::RoleWriteView;

use actix_web::http::StatusCode;
use actix_web::{put, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PutError {
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for PutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for PutError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
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

async fn put_role(
    id: u64,
    payload: RoleWriteView,
    state: web::Data<AppState>,
) -> Result<(), PutError> {
    let smart_db = state.get_smart_db();
    // L'existence du rôle vient d'être vérifiée juste au-dessus : pas besoin de la
    // revérifier une deuxième fois avant d'exécuter la mise à jour.
    if !does_role_exist(id, smart_db).await {
        return Err(PutError::NotFound);
    }
    let view = ChangeRoleQueryView::new(
        id,
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );
    smart_db
        .execute(view)
        .await
        .map_err(|_| PutError::DatabaseError)?;
    Ok(())
}

#[utoipa::path(
    put,
    path = "/{id}",
    summary = "Remplacer un rôle",
    description = "Remplace intégralement le nom, la description et le caractère supprimable d'un \
                   rôle existant. Réservé aux administrateurs.\n\n\
                   Contrairement au `PATCH`, tous les champs du corps sont écrits : un champ omis \
                   est écrasé par sa valeur par défaut, pas conservé. Pour une modification \
                   partielle, utiliser `PATCH /api/v1/admin/roles/{id}`.\n\n\
                   La réponse a un corps vide ; relire le rôle via `GET /api/v1/admin/roles/`.",
    request_body(
        content = RoleWriteView,
        description = "Nouvelle définition complète du rôle.",
        example = json!({
            "name": "agent",
            "description": "Agent municipal habilité à instruire les dossiers",
            "can_be_deleted": true
        })
    ),
    responses(
        (
            status = 200,
            description = "Rôle remplacé. Corps vide.",
        ),
        (
            status = 400,
            description = "Corps JSON malformé, champ obligatoire absent, ou `id` du chemin qui n'est pas un entier.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `name`")
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
#[put("/{id}")]
pub async fn admin_put_role(
    id: web::Path<u64>,
    payload: web::Json<RoleWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PutError> {
    put_role(id.into_inner(), payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
