use crate::database::ressources::add_access_to_user::AddAccessToUserQueryView;
use crate::database::ressources::get_ressource_type_id::GetRessourceTypeIdQueryView;
use crate::database::rights::get_permission_id::{GetPermissionIdQueryView, PermissionAction};
use crate::endpoints::v1::ressources::add_access::view::AddAccessView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum AddAccessError {
    BadRequest,
}

impl std::fmt::Display for AddAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddAccessError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for AddAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            AddAccessError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_request_view(
    smart_db: &SmartDatabase,
    request_view: AddAccessView,
) -> Result<AddAccessToUserQueryView, AddAccessError> {
    let ressource_type_id: i32 = smart_db
        .fetch_scalar(&GetRessourceTypeIdQueryView::new(
            request_view.ressource_type(),
        ))
        .await
        .map_err(|_| AddAccessError::BadRequest)?;
    let ressource_type_id = ressource_type_id as u64;

    let access_type_id: i32 = smart_db
        .fetch_scalar(&GetPermissionIdQueryView::new(
            ressource_type_id,
            PermissionAction::from(request_view.access_type().as_str().to_string()),
        ))
        .await
        .map_err(|_| AddAccessError::BadRequest)?;
    let access_type_id = access_type_id as u64;

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
    let smart_db = state.get_smart_db();

    let view = get_request_view(smart_db, view)
        .await
        .map_err(|_| AddAccessError::BadRequest)?;

    smart_db
        .execute(view)
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
