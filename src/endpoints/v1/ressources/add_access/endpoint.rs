use crate::database::ressources::add_access_to_user::{
    add_access_to_user_query, AddAccessToUserQueryView,
};
use crate::database::ressources::get_ressource_type_id::{
    get_ressource_type_id_query, GetRessourceTypeIdQueryView,
};
use crate::database::rights::get_permission_id::{
    get_permission_id_query, GetPermissionIdQueryView, PermissionAction,
};
use crate::endpoints::v1::ressources::add_access::view::AddAccessView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq)]
enum AddAccessError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for AddAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddAccessError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            AddAccessError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for AddAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            AddAccessError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            AddAccessError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_request_view(
    pool: PgPool,
    request_view: AddAccessView,
) -> Result<AddAccessToUserQueryView, AddAccessError> {
    let ressource_type_id = get_ressource_type_id_query(
        GetRessourceTypeIdQueryView::new(request_view.ressource_type()),
        pool.clone(),
    )
    .await
    .map_err(|_| AddAccessError::BadRequest)?;

    let access_type_id = get_permission_id_query(
        GetPermissionIdQueryView::new(
            ressource_type_id,
            PermissionAction::from(request_view.access_type().as_str().to_string()),
        ),
        pool.clone(),
    )
    .await
    .map_err(|_| AddAccessError::BadRequest)?;

    Ok(AddAccessToUserQueryView::new(
        request_view.user_id(),
        request_view.resource_id(),
        ressource_type_id,
        access_type_id,
    ))
}

async fn add_access_to_ressource(
    state: web::Data<AppState>,
    view: AddAccessView,
) -> Result<(), AddAccessError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(AddAccessError::DatabaseError),
    };

    let view = get_request_view(pool.clone(), view)
        .await
        .map_err(|_| AddAccessError::BadRequest)?;

    add_access_to_user_query(view, pool)
        .await
        .map_err(|_| AddAccessError::BadRequest)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/add_access",
    responses(
        (status = 200, description = "Access added successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Ressources",
    security(
        ("jwt" = [])
    )
)]
#[post("/add_access")]
pub async fn add_access(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    view: web::Json<AddAccessView>,
) -> Result<impl Responder, AddAccessError> {
    add_access_to_ressource(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("Access added successfully"))
}
