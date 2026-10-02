use crate::database::groups::get_user_groups::GetUserGroupsQuerView;
use crate::endpoints::db_error;
use crate::endpoints::v1::groups::get::view::GetGroupsResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetGroupsError {
    DatabaseError,
}

impl std::fmt::Display for GetGroupsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetGroupsError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_groups(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<GetGroupsResultView, GetGroupsError> {
    let groups = state
        .get_smart_db()
        .fetch_all(&GetUserGroupsQuerView::new(user.id))
        .await
        .map_err(|e| {
            db_error::log("list the caller's groups", &e);
            GetGroupsError::DatabaseError
        })?;

    Ok(groups.into())
}

#[utoipa::path(
    get,
    path = "",
    summary = "Lister ses groupes",
    description = "Renvoie les groupes dont l'utilisateur porté par le JWT est membre, qu'il en \
                   soit propriétaire ou simple participant. Il n'existe pas d'endpoint listant \
                   tous les groupes de la plateforme.\n\n\
                   La liste est vide si l'utilisateur n'appartient à aucun groupe.",
    responses(
        (
            status = 200,
            description = "Groupes de l'utilisateur connecté.",
            body = GetGroupsResultView,
            example = json!({
                "groups": [
                    { "id": 3, "owner_id": 2, "name": "Service urbanisme", "description": "Instruction des permis de construire" },
                    { "id": 7, "owner_id": 5, "name": "Astreinte week-end", "description": null }
                ]
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
            description = "Database error, logged by the server.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_groups(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, GetGroupsError> {
    let result = trigger_get_groups(user, state).await?;
    Ok(HttpResponse::Ok().json(result))
}
