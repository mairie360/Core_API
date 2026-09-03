use crate::database::groups::get_user_groups::GetUserGroupsQuerView;
use crate::endpoints::v1::groups::get::view::GetGroupsResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetGroupsError {
    BadRequest,
}

impl std::fmt::Display for GetGroupsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetGroupsError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for GetGroupsError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetGroupsError::BadRequest => StatusCode::BAD_REQUEST,
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
        .map_err(|_| GetGroupsError::BadRequest)?;

    Ok(groups.into())
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, body = GetGroupsResultView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
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
