use crate::database::roles::change_role::ChangeRoleQueryView;
use crate::database::roles::does_role_exist::DoesRoleExistQueryView;
use crate::database::roles::is_rename_forbidden::IsRenameForbiddenQueryView;
use crate::endpoints::v1::admin::roles::view::RoleWriteView;

use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{put, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PutError {
    NotFound,
    Duplicate,
    ProtectedName,
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
            Self::Duplicate => {
                write!(f, "A role with this name already exists.")
            }
            Self::ProtectedName => {
                write!(f, "System roles cannot be renamed.")
            }
        }
    }
}

impl ResponseError for PutError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Duplicate => StatusCode::CONFLICT,
            Self::ProtectedName => StatusCode::FORBIDDEN,
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
    if let Some(name) = Some(payload.name()) {
        // Checked here: the `protect_role_names` trigger would otherwise abort the update (500).
        let forbidden: bool = smart_db
            .fetch_scalar(&IsRenameForbiddenQueryView::new(id, name))
            .await
            .map_err(|_| PutError::DatabaseError)?;
        if forbidden {
            return Err(PutError::ProtectedName);
        }
    }
    let view = ChangeRoleQueryView::new(
        id,
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );
    smart_db.execute(view).await.map_err(|e| match e {
        ApiLibError::Database(DbError::UniqueViolation(_)) => PutError::Duplicate,
        _ => PutError::DatabaseError,
    })?;
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
            description = "Malformed JSON body, missing field, `id` in the path that is not an integer, or a field breaking its rules: `name` 1 to 64 characters, not blank, no control character, no `<` or `>`; `description` at most 1000 characters, no `<` or `>`, no control character other than line breaks and tabs.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `name`: must be at most 64 characters")
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
            description = "The user is authenticated but is not an administrator (`Forbidden: User is not an admin.`), or `name` renames a system role (`can_be_deleted = false`, e.g. `Admin`, `Maire`), whose names are reserved (`System roles cannot be renamed.`).",
            body = String,
            content_type = "text/plain",
            example = json!("System roles cannot be renamed.")
        ),
        (
            status = 404,
            description = "Aucun rôle ne porte cet identifiant.",
            body = String,
            content_type = "text/plain",
            example = json!("The requested resource was not found.")
        ),
        (
            status = 409,
            description = "`name` is already used by another role.",
            body = String,
            content_type = "text/plain",
            example = json!("A role with this name already exists.")
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
        ("id" = u64, Path, description = "Role id. The base roles (1 to 5: Admin, Maire, Responsable, User, Guest) are protected; the roles created afterwards start at 6.", example = 6)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[put("/{id}")]
pub async fn admin_put_role(
    id: web::Path<u64>,
    payload: ValidatedJson<RoleWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PutError> {
    put_role(id.into_inner(), payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
