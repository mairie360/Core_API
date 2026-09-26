use crate::database::admin::reset_password::AdminResetPasswordQueryView;
use crate::endpoints::v1::admin::users::id::password::view::{
    AdminResetPasswordView, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH,
};
use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    InvalidPassword,
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPassword => write!(
                f,
                "The password must contain between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH} characters"
            ),
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidPassword => StatusCode::BAD_REQUEST,
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn reset_password(
    state: web::Data<AppState>,
    user_id: u64,
    view: AdminResetPasswordView,
) -> Result<(), ResetPasswordError> {
    let length = view.new_password().chars().count();
    if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH).contains(&length) {
        return Err(ResetPasswordError::InvalidPassword);
    }

    let hashed_password = hash_password(view.new_password()).map_err(|error| {
        eprintln!("{error:?}");
        ResetPasswordError::DatabaseError
    })?;

    let updated: bool = state
        .get_smart_db()
        .fetch_scalar(&AdminResetPasswordQueryView::new(user_id, &hashed_password))
        .await
        .map_err(|error| {
            eprintln!("{error:?}");
            ResetPasswordError::DatabaseError
        })?;

    if updated {
        Ok(())
    } else {
        Err(ResetPasswordError::UnknownUser)
    }
}

#[utoipa::path(
    patch,
    path = "password",
    summary = "Réinitialiser le mot de passe d'un utilisateur (administration)",
    description = "Attribue un nouveau mot de passe à un compte, sans passer par l'e-mail de \
                   réinitialisation. Sert à débloquer un utilisateur qui n'a plus accès à sa boîte \
                   mail. Réservé aux administrateurs.\n\n\
                   Le mot de passe doit contenir entre 8 et 255 caractères — comptés en \
                   caractères, pas en octets. La réponse a un corps vide.",
    params(
        ("userId" = u64, Path, description = "Identifiant de l'utilisateur.", example = 42)
    ),
    request_body(
        content = AdminResetPasswordView,
        description = "Nouveau mot de passe, de 8 à 255 caractères.",
        example = json!({ "new_password": "NouveauMotDePasse!123" })
    ),
    responses(
        (
            status = 204,
            description = "Mot de passe réinitialisé. Corps vide.",
        ),
        (
            status = 400,
            description = "Corps JSON malformé, ou mot de passe de moins de 8 ou de plus de 255 caractères.",
            body = String,
            content_type = "text/plain",
            example = json!("The password must contain between 8 and 255 characters")
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
            description = "Aucun utilisateur ne porte cet identifiant.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de l'enregistrement du mot de passe.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/password")]
pub async fn admin_reset_user_password(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<AdminResetPasswordView>,
) -> Result<impl Responder, ResetPasswordError> {
    reset_password(state, path.into_inner(), view.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}
