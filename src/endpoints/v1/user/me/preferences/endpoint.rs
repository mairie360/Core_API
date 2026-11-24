use crate::database::users::get_preferences::{GetPreferencesQueryView, UserPreferences};
use crate::database::users::patch_preferences::PatchPreferencesQueryView;
use crate::endpoints::v1::user::me::preferences::view::PatchPreferencesView;
use actix_web::http::StatusCode;
use actix_web::{get, patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PreferencesError {
    BadRequest(String),
    DatabaseError,
}

impl std::fmt::Display for PreferencesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message) => write!(f, "{message}"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for PreferencesError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    get,
    path = "/preferences/",
    summary = "Read one's display preferences",
    description = "Returns the display preferences of the user carried by the JWT. A user who \
                   never saved any preference gets every display field `null`: the application \
                   defaults apply.\n\n\
                   `onboarding_completed` tells whether the user finished or skipped the welcome \
                   tour of the Dashboard: `false` while not done yet (a new account, or after a \
                   reset), `true` once done. It is always a boolean, never `null`.",
    responses(
        (
            status = 200,
            description = "Stored display preferences; `null` fields use the application default.",
            body = UserPreferences,
            example = json!({
                "theme": "dark",
                "font_family": null,
                "font_size": 16,
                "density": null,
                "language": "fr",
                "timezone": "Europe/Paris",
                "date_format": null,
                "home_page": null,
                "auto_open_notifications": false,
                "autosave": true,
                "onboarding_completed": true
            })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while reading the preferences.",
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
#[get("/preferences/")]
pub async fn get_my_preferences(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PreferencesError> {
    let preferences: UserPreferences = state
        .get_smart_db()
        .fetch_one(&GetPreferencesQueryView::new(auth_user.id))
        .await
        .map_err(|e| {
            tracing::error!("Error: {e:?}");
            PreferencesError::DatabaseError
        })?;
    Ok(HttpResponse::Ok().json(preferences))
}

#[utoipa::path(
    patch,
    path = "/preferences/",
    summary = "Update one's display preferences",
    description = "Partially updates the display preferences of the user carried by the JWT, \
                   creating them on the first call, and returns the stored state.\n\n\
                   For every field, absent keeps the stored value, `null` resets it to the \
                   application default and a value stores it.\n\n\
                   `onboarding_completed` accepts `true` (the welcome tour is done: finished or \
                   skipped; idempotent) or `null` (reset: the tour shows again and reads `false`), \
                   never `false`. \
                   The other preferences are left untouched when only this key is sent.",
    request_body(
        content = PatchPreferencesView,
        description = "Fields to change. All optional: absent keeps the stored value, `null` resets it to the default.",
        example = json!({ "theme": "dark", "font_size": 16, "date_format": null })
    ),
    responses(
        (
            status = 200,
            description = "Stored display preferences after the update.",
            body = UserPreferences,
            example = json!({
                "theme": "dark",
                "font_family": null,
                "font_size": 16,
                "density": null,
                "language": "fr",
                "timezone": "Europe/Paris",
                "date_format": null,
                "home_page": null,
                "auto_open_notifications": false,
                "autosave": true,
                "onboarding_completed": true
            })
        ),
        (
            status = 400,
            description = "Malformed JSON body, field of an unexpected type, or a value outside the ones the application understands (MAIR-479): `theme` is `light`, `dark` or `system`; `font_size` 1 to 32767; `font_family` is `system`, `geist`, `atkinson` or `lora` (Settings V2, MAIR-1128), or one of the earlier `Marianne`, `Arial`, `Verdana`, `OpenDyslexic`, `system-ui`; `density` is `compact`, `normal` or `comfortable`; `date_format` is `DD/MM/YYYY`, `MM/DD/YYYY` or `YYYY-MM-DD`; `language` a language code such as `fr` or `fr-FR`; `timezone` an IANA time zone such as `Europe/Paris` (case-sensitive); `home_page` a path such as `/dashboard` (lowercase letters, digits, `-`, `_`, `/`, no `//`, 128 characters at most). `onboarding_completed` is `true` or `null`, `false` is refused. The body names the first invalid field. Nothing is saved.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `density`: must be one of `compact`, `normal`, `comfortable`")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while saving the preferences.",
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
#[patch("/preferences/")]
pub async fn patch_my_preferences(
    state: web::Data<AppState>,
    view: web::Json<PatchPreferencesView>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PreferencesError> {
    let view = view.into_inner();
    view.validate().map_err(PreferencesError::BadRequest)?;
    let stored: Vec<UserPreferences> = state
        .get_smart_db()
        .fetch_all(&PatchPreferencesQueryView::new(
            auth_user.id,
            &view.as_patch(),
        ))
        .await
        .map_err(|e| {
            tracing::error!("Error: {e:?}");
            PreferencesError::DatabaseError
        })?;
    stored
        .into_iter()
        .next()
        .map_or(Err(PreferencesError::DatabaseError), |stored| {
            Ok(HttpResponse::Ok().json(stored))
        })
}
