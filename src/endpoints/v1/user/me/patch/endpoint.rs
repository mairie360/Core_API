use crate::database::ids::id_to_sql;
use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::auth::user_password::{GetUserPasswordQueryView, UserPassword};
use crate::database::users::patch_user::PatchUserQueryView;
use crate::endpoints::v1::auth::login::endpoint::is_password_valid;
use crate::endpoints::v1::user::me::patch::view::PatchMeView;
use crate::endpoints::validation::ValidatedJson;

#[derive(Debug, Clone, PartialEq)]
enum PatchMeError {
    BadRequest(String),
    EmailAlreadyUsed,
    DatabaseError,
    WrongPassword,
}

impl std::fmt::Display for PatchMeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message) => write!(f, "{message}"),
            Self::EmailAlreadyUsed => {
                write!(f, "Another account already uses this e-mail address.")
            }
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::WrongPassword => write!(f, "The current password is incorrect."),
        }
    }
}

impl ResponseError for PatchMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::EmailAlreadyUsed => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::WrongPassword => StatusCode::FORBIDDEN,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Changing the e-mail address requires the current password (MAIR-390). Accounts without a
/// password (Keycloak only) cannot change it here.
async fn check_current_password(
    state: &AppState,
    user_id: u64,
    password: &str,
) -> Result<(), PatchMeError> {
    let stored: UserPassword = state
        .get_smart_db()
        .fetch_one(&GetUserPasswordQueryView::new(user_id))
        .await
        .map_err(|e| {
            tracing::error!("Patch me DB Error: {e}");
            PatchMeError::DatabaseError
        })?;
    match stored.password() {
        Some(hash) if is_password_valid(id_to_sql(user_id), password, hash).await => Ok(()),
        _ => Err(PatchMeError::WrongPassword),
    }
}

async fn trigger_patch_me(
    state: web::Data<AppState>,
    view: PatchMeView,
    user_id: u64,
) -> Result<(), PatchMeError> {
    if view.email().is_some() {
        check_current_password(&state, user_id, view.current_password().unwrap_or_default())
            .await?;
    }
    let phone = view
        .phone_change()
        .map_err(|e| PatchMeError::BadRequest(e.to_string()))?;
    let db_view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        view.email(),
        &phone,
        None,
    );
    if !db_view.is_noop() {
        state
            .get_smart_db()
            .execute(db_view)
            .await
            .map_err(|e| match e {
                ApiLibError::Database(DbError::UniqueViolation(_)) => {
                    PatchMeError::EmailAlreadyUsed
                }
                e => {
                    tracing::error!("Error: {e:?}");
                    PatchMeError::DatabaseError
                }
            })?;
    }
    Ok(())
}

#[utoipa::path(
    patch,
    path = "/",
    summary = "Modifier son propre profil",
    description = "Met à jour l'état civil, l'adresse e-mail ou le téléphone de l'utilisateur \
                   porté par le JWT. Modification partielle : seuls les champs présents dans le \
                   corps sont écrits, les autres restent inchangés.\n\n\
                   Un corps vide est accepté et ne déclenche aucune écriture : la réponse reste \
                   `200`. Un champ `null` est ignoré, sauf `phone` : `null` ou `\"\"` supprime \
                   le téléphone (MAIR-480).\n\n\
                   Le téléphone s'envoie tel que saisi, au format national avec \
                   `phone_country` (`\"06 12 34 56 78\"` + `\"FR\"`) ou en E.164 \
                   (`\"+33612345678\"`). Il est validé selon le plan de numérotation du pays et \
                   relu en E.164 par `GET /api/v1/user/me/`.\n\n\
                   Le mot de passe et les rôles ne se modifient pas ici : passer par \
                   `/api/v1/auth/forgot_password` pour le mot de passe et par \
                   `/api/v1/admin/users/` pour les rôles. La réponse a un corps vide ; il faut \
                   rappeler `GET /api/v1/user/me/` pour relire le profil.",
    request_body(
        content = PatchMeView,
        description = "Fields to change, all optional: absent or `null` is ignored, except `phone` where `null` or `\"\"` removes the phone.",
        examples(
            ("Change the phone" = (value = json!({ "first_name": "Jean", "phone": "06 12 34 56 78", "phone_country": "FR" }))),
            ("International number" = (value = json!({ "phone": "+32 470 12 34 56" }))),
            ("Remove the phone" = (value = json!({ "phone": null })))
        )
    ),
    responses(
        (
            status = 200,
            description = "Profil mis à jour, ou rien à mettre à jour. Corps vide.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, field of an unexpected type, or a field breaking its rules: `first_name` / `last_name` 1 to 64 characters, not blank, no control character; `email` a valid address of at most 320 characters; `phone` not a valid number of `phone_country` (or a national number without `phone_country`, 32 characters at most); `phone_country` not an ISO 3166-1 alpha-2 code, or sent without `phone`; `current_password` missing while `email` is sent. The body names the first invalid field.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `phone`: is not a phone number of the selected country")
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
            description = "`email` was sent with a wrong `current_password`, or the account has no password (Keycloak only). Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("The current password is incorrect.")
        ),
        (
            status = 409,
            description = "`email` is already used by another account.",
            body = String,
            content_type = "text/plain",
            example = json!("Another account already uses this e-mail address.")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la mise à jour du profil.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn patch_me(
    state: web::Data<AppState>,
    view: ValidatedJson<PatchMeView>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PatchMeError> {
    trigger_patch_me(state, view.into_inner(), auth_user.id).await?;
    Ok(HttpResponse::Ok())
}
