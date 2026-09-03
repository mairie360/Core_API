use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_user_from_group::DeleteUserFromGroupQueryView;
use crate::database::groups::is_user_member::IsUserMemberQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserFromGroupError {
    BadRequest,
    UnknowUser,
}

impl std::fmt::Display for DeleteUserFromGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserFromGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
            DeleteUserFromGroupError::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for DeleteUserFromGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserFromGroupError::BadRequest => StatusCode::BAD_REQUEST,
            DeleteUserFromGroupError::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user_from_group(
    state: web::Data<AppState>,
    group_id: u64,
    user_id: u64,
) -> Result<(), DeleteUserFromGroupError> {
    let smart_db = state.get_smart_db();

    let user_check_view = IsUserMemberQueryView::new(group_id, user_id);
    let result: bool = smart_db
        .fetch_scalar(&user_check_view)
        .await
        .map_err(|_| DeleteUserFromGroupError::UnknowUser)?;
    if !result {
        return Err(DeleteUserFromGroupError::UnknowUser);
    }

    let db_view = DeleteUserFromGroupQueryView::new(group_id, user_id);
    smart_db
        .execute(db_view)
        .await
        .map_err(|_| DeleteUserFromGroupError::BadRequest)?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "/",
    params(
        ("group_id" = u64, Path, description = "ID du groupe"),
        ("user_id" = u64, Path, description = "ID de l'utilisateur")
    ),
    responses(
        (status = 204, description = "User deleted from group successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn remove_user_from_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(u64, u64)>,
) -> Result<impl Responder, DeleteUserFromGroupError> {
    let (group_id, user_id) = path.into_inner();
    delete_user_from_group(state, group_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
