use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::state::AppState;

use crate::{
    database::users::patch_user::PatchUserQueryView,
    endpoints::v1::admin::users::id::patch::view::PatchUserView,
};

#[derive(Debug, Clone, PartialEq)]
enum PatchUserError {
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for PatchUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for PatchUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn patch_user(
    state: web::Data<AppState>,
    user_id: u64,
    view: PatchUserView,
) -> Result<(), PatchUserError> {
    let hashed_password = view
        .password()
        .map(hash_password)
        .transpose()
        .map_err(|e| {
            eprintln!("Password hashing error: {e}");
            PatchUserError::DatabaseError
        })?;

    let view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        view.phone_number(),
        hashed_password.as_deref(),
    );
    if !view.is_noop() {
        state
            .get_smart_db()
            .execute(view)
            .await
            .map_err(|_| PatchUserError::UnknownUser)?;
    }

    Ok(())
}

#[utoipa::path(
    patch,
    path = "",
    summary = "Modifier un utilisateur (administration)",
    description = "Met à jour partiellement un compte : seuls les champs présents dans le corps \
                   sont écrits. Réservé aux administrateurs.\n\n\
                   Contrairement à `PATCH /api/v1/user/me/`, un administrateur peut aussi changer \
                   le mot de passe ici. Pour une réinitialisation qui applique les règles de \
                   longueur, préférer `PATCH /api/v1/admin/users/{userId}/password`, qui les \
                   vérifie : le champ `password` de cet endpoint est écrit sans validation.\n\n\
                   Un corps vide est accepté et ne déclenche aucune écriture. Comme l'existence du \
                   compte n'est pas vérifiée au préalable, un `userId` inconnu répond alors `200`.",
    params(
        ("userId" = u64, Path, description = "Identifiant de l'utilisateur.", example = 42)
    ),
    request_body(
        content = PatchUserView,
        description = "Champs à modifier. Tous facultatifs ; un champ absent ou `null` est ignoré.",
        example = json!({
            "email": "j.dupont@mairie360.fr",
            "phone_number": "0798765432"
        })
    ),
    responses(
        (
            status = 200,
            description = "Compte mis à jour, ou rien à mettre à jour.",
            body = String,
            content_type = "text/plain",
            example = json!("User patched successfully!")
        ),
        (
            status = 400,
            description = "Corps JSON malformé, ou `userId` du chemin qui n'est pas un entier.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: invalid type: integer `42`, expected a string")
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
            description = "L'écriture a échoué : `userId` inconnu, ou adresse e-mail déjà prise par un autre compte.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Le champ `password` était présent mais n'a pas pu être haché (erreur interne argon2).",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        ),
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn admin_patch_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<PatchUserView>,
) -> Result<impl Responder, PatchUserError> {
    patch_user(state, path.into_inner(), view.into_inner()).await?;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
