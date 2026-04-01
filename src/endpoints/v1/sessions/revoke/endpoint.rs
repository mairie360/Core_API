use crate::database::queries::sessions::revoke_session_by_token::{
    revoke_session_by_token_query, RevokeSessionByTokenQueryView,
};
use crate::endpoints::v1::sessions::revoke::request_view::RevokeRequestView;
use crate::endpoints::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};

use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum AboutError {
    DatabaseError,
}

impl std::fmt::Display for AboutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AboutError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for AboutError {
    fn status_code(&self) -> StatusCode {
        match self {
            AboutError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn revoke_request(
    user: AuthenticatedUser,
    view: RevokeRequestView,
    state: web::Data<AppState>,
) -> Result<(), AboutError> {
    let user_id = user.id;

    match revoke_session_by_token_query(
        RevokeSessionByTokenQueryView::new(user_id, view.token_id()),
        state.db_pool.clone(),
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(_) => Err(AboutError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "revoke",
    request_body = RevokeRequestView,
    responses(
        (status = 200, description = "Token revoked successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[post("/revoke")]
pub async fn revoke(
    user: AuthenticatedUser,
    body: web::Json<RevokeRequestView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AboutError> {
    let view = body.into_inner();

    revoke_request(user, view, state)
        .await
        .map(|_| HttpResponse::Ok())
}
