use crate::database::auth::is_first_time::{is_first_time_query, IsFirstTimeQueryView};
use crate::database::auth::unset_first_connection::{
    unset_first_connection_query, UnsetFirstConnectionQueryView,
};
use crate::endpoints::v1::auth::force_change_password::view::ForceChangePasswordView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::redis::simple_key::secured::handle_secure_get;
use mairie360_api_lib::pool::AppState;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq)]
enum ForceChanhePasswordError {
    DatabaseError,
    Forbidden,
    Unauthorized,
}

impl std::fmt::Display for ForceChanhePasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForceChanhePasswordError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            ForceChanhePasswordError::Forbidden => {
                write!(f, "Unknown user token")
            }
            ForceChanhePasswordError::Unauthorized => {
                write!(f, "Unauthorized")
            }
        }
    }
}

impl ResponseError for ForceChanhePasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            ForceChanhePasswordError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ForceChanhePasswordError::Forbidden => StatusCode::FORBIDDEN,
            ForceChanhePasswordError::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_id(state: &AppState, token: &str) -> Option<u64> {
    println!("{}", format!("{}/first_connection_id", token));
    match handle_secure_get(
        state.get_redis_conn().await.unwrap(),
        &format!("{}/first_connection_id", token),
    )
    .await
    {
        Ok(id) => Some(id.parse().unwrap()),
        Err(_) => None,
    }
}

async fn is_first_time(pool: PgPool, user_id: u64) -> bool {
    is_first_time_query(IsFirstTimeQueryView::new(user_id), pool)
        .await
        .unwrap_or(false)
}

async fn change_password(
    pool: PgPool,
    user_id: u64,
    new_password: &str,
) -> Result<(), ForceChanhePasswordError> {
    unset_first_connection_query(
        UnsetFirstConnectionQueryView::new(user_id, new_password),
        pool,
    )
    .await
    .map_err(|_| ForceChanhePasswordError::DatabaseError)
}

async fn force_change_password_trigger(
    state: web::Data<AppState>,
    view: ForceChangePasswordView,
) -> Result<(), ForceChanhePasswordError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(ForceChanhePasswordError::DatabaseError),
    };

    let user_id = match get_user_id(&state, view.token()).await {
        Some(user_id) => user_id,
        None => return Err(ForceChanhePasswordError::Forbidden),
    };

    if !is_first_time(pool.clone(), user_id).await {
        return Err(ForceChanhePasswordError::Unauthorized);
    }

    change_password(pool.clone(), user_id, view.new_password()).await?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/",
    responses(
        (status = 200, description = "Password changed successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Unknown user token"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
#[post("/force_change_password")]
pub async fn force_change_password(
    state: web::Data<AppState>,
    body: web::Json<ForceChangePasswordView>,
) -> Result<impl Responder, ForceChanhePasswordError> {
    force_change_password_trigger(state, body.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
