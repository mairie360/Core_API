use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

use crate::database::groups::delete_user_from_group::{
    delete_user_from_group_query, DeleteUserFromGroupQueryView,
};
use crate::database::groups::does_group_exist::{does_group_exist_query, DoesGroupExistQuerView};
use crate::database::groups::is_user_member::{is_user_member_query, IsUserMemberQueryView};

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserFromGroupError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for DeleteUserFromGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserFromGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            DeleteUserFromGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for DeleteUserFromGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserFromGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            DeleteUserFromGroupError::BadRequest => StatusCode::BAD_REQUEST,
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
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(DeleteUserFromGroupError::DatabaseError),
    };

    let check_view = DoesGroupExistQuerView::new(group_id as u64);
    let result = does_group_exist_query(check_view, pool.clone())
        .await
        .map_err(|_| DeleteUserFromGroupError::BadRequest)?;
    if !result {
        return Err(DeleteUserFromGroupError::BadRequest);
    }

    let user_check_view = IsUserMemberQueryView::new(group_id, user_id);
    let result = is_user_member_query(user_check_view, pool.clone())
        .await
        .map_err(|_| DeleteUserFromGroupError::BadRequest)?;
    if !result {
        return Err(DeleteUserFromGroupError::BadRequest);
    }

    let db_view = DeleteUserFromGroupQueryView::new(group_id, user_id);
    delete_user_from_group_query(db_view, pool)
        .await
        .map_err(|_| DeleteUserFromGroupError::BadRequest)?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "/",
    responses(
        (status = 204, description = "User deleted from group successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn delete(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(u64, u64)>,
) -> Result<impl Responder, DeleteUserFromGroupError> {
    let (group_id, user_id) = path.into_inner();
    delete_user_from_group(state, group_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
