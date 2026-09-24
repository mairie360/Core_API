use crate::database::roles::does_role_exist::DoesRoleExistQueryView;
use crate::database::roles::is_rename_forbidden::IsRenameForbiddenQueryView;
use crate::database::roles::patch_role::PatchRoleQueryView;
use crate::endpoints::v1::admin::roles::patch::view::PatchView;

use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchError {
    NotFound,
    Duplicate,
    ProtectedName,
    DatabaseError,
}

impl std::fmt::Display for PatchError {
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

impl ResponseError for PatchError {
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

async fn patch_role(
    id: u64,
    payload: PatchView,
    state: web::Data<AppState>,
) -> Result<(), PatchError> {
    let smart_db = state.get_smart_db();
    if !does_role_exist(id, smart_db).await {
        return Err(PatchError::NotFound);
    }
    if let Some(name) = payload.name().as_deref() {
        // Checked here: the `protect_role_names` trigger would otherwise abort the update (500).
        let forbidden: bool = smart_db
            .fetch_scalar(&IsRenameForbiddenQueryView::new(id, name))
            .await
            .map_err(|_| PatchError::DatabaseError)?;
        if forbidden {
            return Err(PatchError::ProtectedName);
        }
    }
    let view = PatchRoleQueryView::new(
        id,
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );
    if !view.is_noop() {
        smart_db.execute(view).await.map_err(|e| match e {
            ApiLibError::Database(DbError::UniqueViolation(_)) => PatchError::Duplicate,
            _ => PatchError::DatabaseError,
        })?;
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
            description = "Malformed JSON body, `id` in the path that is not an integer, or a field present and breaking its rules: `name` 1 to 64 characters, not blank, no control character, no `<` or `>`; `description` at most 1000 characters, no `<` or `>`, no control character other than line breaks and tabs.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `description`: must not contain `<` or `>`")
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
#[patch("/{id}")]
pub async fn admin_patch_role(
    id: web::Path<u64>,
    payload: ValidatedJson<PatchView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PatchError> {
    patch_role(id.into_inner(), payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
