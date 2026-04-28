use std::net::IpAddr;

use crate::database::queries::sessions::revoke_session_by_token::{
    revoke_session_by_token_query, RevokeSessionByTokenQueryView,
};
use crate::endpoints::v1::sessions::revoke::request_view::RevokeRequestView;
use crate::endpoints::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};

use mairie360_api_lib::database::queries::is_session_token_valid_query;
use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum RevokeError {
    InvalidToken,
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for RevokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RevokeError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            RevokeError::InvalidToken => {
                write!(f, "Session not found.")
            }
            RevokeError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for RevokeError {
    fn status_code(&self) -> StatusCode {
        match self {
            RevokeError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            RevokeError::InvalidToken => StatusCode::UNAUTHORIZED,
            RevokeError::BadRequest => StatusCode::BAD_REQUEST,
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
    ip_adress: IpAddr,
) -> Result<(), RevokeError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token(), ip_adress);

    let is_valid = is_session_token_valid_query(db_view, state.db_pool.clone()).await;

    let db_view = match is_valid {
        Ok(true) => RevokeSessionByTokenQueryView::new(user_id, &view.refresh_token()),
        Ok(false) => return Err(RevokeError::InvalidToken),
        Err(_) => return Err(RevokeError::DatabaseError),
    };

    match revoke_session_by_token_query(db_view, state.db_pool.clone()).await {
        Ok(_) => Ok(()),
        Err(_) => Err(RevokeError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "revoke",
    request_body = RevokeRequestView,
    responses(
        (status = 200, description = "Token revoked successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized, invalid token or user not found"),
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
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RevokeError> {
    let view = match body.into_inner().try_into() {
        Ok(view) => view,
        Err(_) => return Err(RevokeError::BadRequest),
    };

    let ip_adress = request
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse()
        .unwrap();

    revoke_request(user, view, state, ip_adress)
        .await
        .map(|_| HttpResponse::Ok().body("Session revoked successfully"))
}
