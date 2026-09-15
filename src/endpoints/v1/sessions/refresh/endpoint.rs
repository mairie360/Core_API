use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::state::AppState;

use crate::endpoints::v1::sessions::refresh::request_view::RefreshRequestView;
use mairie360_api_lib::security::AuthenticatedUser;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq)]
enum RefreshError {
    DatabaseError,
    InvalidToken,
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "Session not found"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for RefreshError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn refresh_request(
    user: AuthenticatedUser,
    ip_adress: IpAddr,
    view: RefreshRequestView,
    state: web::Data<AppState>,
) -> Result<String, RefreshError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token(), ip_adress);

    let is_valid: Result<bool, _> = state.get_smart_db().fetch_scalar(&db_view).await;

    match is_valid {
        // TODO: cf. login/endpoint.rs::generate_session — rôle non encore exploité par la lib.
        Ok(true) => generate_jwt(&user_id.to_string(), "").map_err(|_| RefreshError::DatabaseError),
        Ok(false) => Err(RefreshError::InvalidToken),
        Err(_) => Err(RefreshError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "refresh",
    request_body = RefreshRequestView,
    responses(
        (status = 200, description = "Token refreshed successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized, invalid token or user not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
)]
#[post("/refresh")]
// actix-web exécute chaque handler sur un runtime single-threaded par worker : la future
// n'a pas besoin d'être Send même si elle retient un HttpRequest (non-Send) à travers un
// .await, contrairement à ce que suppose ce lint pedantic.
#[allow(clippy::future_not_send)]
pub async fn refresh(
    user: AuthenticatedUser,
    body: web::Json<RefreshRequestView>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RefreshError> {
    let view = body.into_inner();

    let ip_adress = request
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse()
        .unwrap();

    let new_jwt = refresh_request(user, ip_adress, view, state).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {new_jwt}")))
        .body("JWT refreshed successfully"))
}
