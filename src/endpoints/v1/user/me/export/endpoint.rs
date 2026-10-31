use crate::endpoints::export_data::{self, ExportError};
use actix_web::{get, web, HttpResponse, Responder};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[utoipa::path(
    get,
    path = "/export",
    summary = "Export one's own data (GDPR)",
    description = "Right of access and portability (GDPR art. 15 and 20, MAIR-289): returns, as \
                   JSON, the account of the JWT's user and every row the schema links to it \
                   (memberships, sessions, messages, events, tasks, audit entries...), grouped by \
                   `\"<table>.<column>\"`. Password and token hashes are left out. Built by \
                   `export_user_data()` of the database.",
    responses(
        (status = 200, description = "Export of the caller's data.", body = Object, example = json!(export_data::example())),
        (status = 401, description = "`Authorization` header missing, invalid or expired JWT, or revoked session.", body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 404, description = "The JWT's user no longer exists.", body = String, content_type = "text/plain", example = json!("Unknown user")),
        (status = 500, description = "Database error while building the export.", body = String, content_type = "text/plain", example = json!("An error occurred while accessing the database."))
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[get("/export")]
pub async fn export_my_data(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, ExportError> {
    let export = export_data::export_user_data(&state, auth_user.id).await?;
    Ok(HttpResponse::Ok().json(export))
}
