use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::export_data::{self, ExportError};
use actix_web::{get, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[utoipa::path(
    get,
    path = "export",
    summary = "Export the data of a user (administration, GDPR)",
    description = "Answers a right of access or portability request (GDPR art. 15 and 20, \
                   MAIR-289) made to the mairie: every row the schema links to the user, archived \
                   or anonymized accounts included, grouped by `\"<table>.<column>\"`. Password and \
                   token hashes are left out. Administrators only.",
    params(("userId" = u64, Path, description = "User id.", example = 42)),
    responses(
        (status = 200, description = "Export of the user's data.", body = Object, example = json!(export_data::example())),
        (status = 400, description = "`userId` in the path is not an integer.", body = String, content_type = "text/plain", example = json!("can not parse \"abc\" to a u64")),
        (status = 401, description = "`Authorization` header missing, invalid or expired JWT, or revoked session.", body = String, content_type = "text/plain", example = json!("Jeton expiré")),
        (status = 403, description = "The user is authenticated but is not an administrator.", body = String, content_type = "text/plain", example = json!("Forbidden: User is not an admin.")),
        (status = 404, description = "No account has this id.", body = String, content_type = "text/plain", example = json!("Unknown user")),
        (status = 500, description = "Database error while building the export.", body = String, content_type = "text/plain", example = json!("An error occurred while accessing the database."))
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[get("/export")]
pub async fn admin_export_user_data(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, ExportError> {
    let export = export_data::export_user_data(&state, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(export))
}
