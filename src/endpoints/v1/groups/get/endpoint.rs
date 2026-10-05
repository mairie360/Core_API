use crate::database::groups::get_user_groups::GetUserGroupsQuerView;
use crate::endpoints::db_error;
use crate::endpoints::pagination::PageQuery;
use crate::endpoints::v1::groups::get::view::GetGroupsResultView;
use crate::endpoints::validation::ValidatedQuery;
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
    page: PageQuery,
) -> Result<GetGroupsResultView, GetGroupsError> {
    let groups = state
        .get_smart_db()
        .fetch_all(&GetUserGroupsQuerView::page(user.id, page))
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
    description = "Returns the groups the JWT's user is a member of, as owner or participant, \
                   sorted by name. No endpoint lists every group of the platform.\n\n\
                   Paginated (MAIR-425): `limit` groups (100 by default, 500 at most) from \
                   `offset`; a page shorter than `limit` is the last one. The list is empty when \
                   the user belongs to no group.",
    params(PageQuery),
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
            status = 400,
            description = "`limit` outside 1 to 500, or `offset` outside 0 to 1000000.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `limit`: must be between 1 and 500")
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
    page: ValidatedQuery<PageQuery>,
) -> Result<impl Responder, GetGroupsError> {
    let result = trigger_get_groups(user, state, page.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
