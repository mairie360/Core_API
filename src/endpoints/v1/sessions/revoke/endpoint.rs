use crate::database::sessions::revoke_session_by_token::RevokeSessionByTokenQueryView;
use crate::endpoints::v1::sessions::revoke::request_view::RevokeRequestView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};

use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum RevokeError {
    InvalidToken,
    DatabaseError,
}

impl std::fmt::Display for RevokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::InvalidToken => {
                write!(f, "Session not found.")
            }
        }
    }
}

impl ResponseError for RevokeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
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
    ip_address: std::net::IpAddr,
) -> Result<(), RevokeError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token(), ip_address);

    let is_valid: Result<bool, _> = state.get_smart_db().fetch_scalar(&db_view).await;

    let db_view = match is_valid {
        Ok(true) => RevokeSessionByTokenQueryView::new(user_id, &view.refresh_token()),
        Ok(false) => return Err(RevokeError::InvalidToken),
        Err(_) => return Err(RevokeError::DatabaseError),
    };

    match state.get_smart_db().execute(db_view).await {
        Ok(()) => Ok(()),
        Err(_) => Err(RevokeError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "revoke",
    request_body = RevokeRequestView,
    responses(
        (status = 200, description = "Token revoked successfully"),
        (status = 401, description = "Unauthorized, invalid token or user not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[post("/revoke")]
// actix-web exécute chaque handler sur un runtime single-threaded par worker : la future
// n'a pas besoin d'être Send même si elle retient un HttpRequest (non-Send) à travers un
// .await, contrairement à ce que suppose ce lint pedantic.
#[allow(clippy::future_not_send)]
pub async fn revoke(
    user: AuthenticatedUser,
    body: web::Json<RevokeRequestView>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RevokeError> {
    let view = body.into_inner();
    // Depuis mairie360_api_lib (MAIR-125), l'IP n'est plus un critère de validité du token : elle
    // n'est lue que pour la signature de la vue, avec le même repli que le login.
    let ip_address = request
        .connection_info()
        .realip_remote_addr()
        .and_then(|ip| ip.parse().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    revoke_request(user, view, state, ip_address)
        .await
        .map(|()| HttpResponse::Ok().body("Session revoked successfully"))
}
