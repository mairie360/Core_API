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
    summary = "Lire ses préférences d'affichage",
    description = "Returns the display preferences of the user carried by the JWT. A user who \
                   never saved any preference gets every field `null`: the application defaults \
                   apply.",
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
                "auto_open_notifications": false
            })
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
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
            eprintln!("Error: {e:?}");
            PreferencesError::DatabaseError
        })?;
    Ok(HttpResponse::Ok().json(preferences))
}

#[utoipa::path(
    patch,
    path = "/preferences/",
    summary = "Modifier ses préférences d'affichage",
    description = "Partially updates the display preferences of the user carried by the JWT, \
                   creating them on the first call, and returns the stored state.\n\n\
                   For every field, absent keeps the stored value, `null` resets it to the \
                   application default and a value stores it.",
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
                "auto_open_notifications": false
            })
        ),
        (
            status = 400,
            description = "Malformed JSON body, field of an unexpected type, or a value breaking its rules: `theme` is `light`, `dark` or `system`; `font_size` 1 to 32767; `font_family` and `home_page` 1 to 128 characters, `timezone` 1 to 64, `density` and `date_format` 1 to 32, `language` 1 to 16, not blank, no control character, no `<` or `>`.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `theme`: must be `light`, `dark` or `system`")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
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
            eprintln!("Error: {e:?}");
            PreferencesError::DatabaseError
        })?;
    stored
        .into_iter()
        .next()
        .map_or(Err(PreferencesError::DatabaseError), |stored| {
            Ok(HttpResponse::Ok().json(stored))
        })
}
