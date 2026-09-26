use crate::database::users::get_notification_settings::{
    GetNotificationSettingsQueryView, UserNotificationSettings,
};
use crate::database::users::patch_notification_settings::PatchNotificationSettingsQueryView;
use crate::endpoints::v1::user::me::notifications::view::PatchNotificationSettingsView;
use actix_web::http::StatusCode;
use actix_web::{get, patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum NotificationSettingsError {
    DatabaseError,
}

impl std::fmt::Display for NotificationSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for NotificationSettingsError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    get,
    path = "/notifications/",
    summary = "Lire ses réglages de notification",
    description = "Returns the notification settings of the user carried by the JWT, per \
                   channel (e-mail, push, desktop) and per module (messages, projects, \
                   calendar). A user who never saved any setting gets every field `null`: the \
                   application defaults apply.",
    responses(
        (
            status = 200,
            description = "Stored notification settings; `null` fields use the application default.",
            body = UserNotificationSettings,
            example = json!({
                "email": true,
                "push": null,
                "desktop": true,
                "messages": true,
                "projects": null,
                "calendar": false
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
            description = "Database error while reading the notification settings.",
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
#[get("/notifications/")]
pub async fn get_my_notification_settings(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, NotificationSettingsError> {
    let settings: UserNotificationSettings = state
        .get_smart_db()
        .fetch_one(&GetNotificationSettingsQueryView::new(auth_user.id))
        .await
        .map_err(|e| {
            eprintln!("Error: {e:?}");
            NotificationSettingsError::DatabaseError
        })?;
    Ok(HttpResponse::Ok().json(settings))
}

#[utoipa::path(
    patch,
    path = "/notifications/",
    summary = "Modifier ses réglages de notification",
    description = "Partially updates the notification settings of the user carried by the JWT, \
                   creating them on the first call, and returns the stored state.\n\n\
                   For every field, absent keeps the stored value, `null` resets it to the \
                   application default and `true` / `false` stores it.",
    request_body(
        content = PatchNotificationSettingsView,
        description = "Fields to change. All optional: absent keeps the stored value, `null` resets it to the default.",
        example = json!({ "push": false, "calendar": null })
    ),
    responses(
        (
            status = 200,
            description = "Stored notification settings after the update.",
            body = UserNotificationSettings,
            example = json!({
                "email": true,
                "push": false,
                "desktop": true,
                "messages": true,
                "projects": null,
                "calendar": null
            })
        ),
        (
            status = 400,
            description = "Malformed JSON body, or a field that is neither a boolean nor `null`.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: invalid type: string \"yes\", expected a boolean")
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
            description = "Database error while saving the notification settings.",
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
#[patch("/notifications/")]
pub async fn patch_my_notification_settings(
    state: web::Data<AppState>,
    view: web::Json<PatchNotificationSettingsView>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, NotificationSettingsError> {
    let stored: Vec<UserNotificationSettings> = state
        .get_smart_db()
        .fetch_all(&PatchNotificationSettingsQueryView::new(
            auth_user.id,
            view.into_inner().as_patch(),
        ))
        .await
        .map_err(|e| {
            eprintln!("Error: {e:?}");
            NotificationSettingsError::DatabaseError
        })?;
    stored
        .into_iter()
        .next()
        .map_or(Err(NotificationSettingsError::DatabaseError), |stored| {
            Ok(HttpResponse::Ok().json(stored))
        })
}
