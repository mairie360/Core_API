use crate::database::groups::get_user_groups::GetUserGroupsQuerView;
use crate::database::roles::get_roles_of_user::GetRolesOfUserQueryView;
use crate::database::users::get_user_by_id::GetUserByIdQueryView;
use crate::email_change;
use crate::endpoints::v1::user::me::get::view::GetMeResponseView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetMeError {
    DatabaseError,
}

impl std::fmt::Display for GetMeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_me(
    state: web::Data<AppState>,
    user_id: u64,
) -> Result<GetMeResponseView, GetMeError> {
    let smart_db = state.get_smart_db();
    let view = GetUserByIdQueryView::new(user_id);
    let result: crate::database::users::get_user_by_id::GetUserByIdQueryResultView =
        smart_db.fetch_one(&view).await.map_err(|e| {
            tracing::error!("Get user DB Error: {e}");
            GetMeError::DatabaseError
        })?;
    let view = GetUserGroupsQuerView::new(user_id);
    let groups = smart_db.fetch_all(&view).await.map_err(|e| {
        tracing::error!("Get user DB Error: {e}");
        GetMeError::DatabaseError
    })?;
    let view = GetRolesOfUserQueryView::new(user_id);
    let role: Vec<crate::database::roles::get_roles_by_id::Role> =
        smart_db.fetch_all(&view).await.map_err(|e| {
            tracing::error!("Get user DB Error: {e}");
            GetMeError::DatabaseError
        })?;

    // A Redis failure only hides the pending change: the profile is still served.
    let pending_email = email_change::pending_email(state.get_redis(), user_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Get me: could not read the pending e-mail change: {e}");
            None
        });

    Ok(GetMeResponseView::new(
        result.first_name(),
        result.last_name(),
        result.email(),
        result.phone_e164(),
        result.phone_country(),
        result.status(),
        role.iter().map(|r| r.name().to_string()).collect(),
        groups,
        pending_email,
    ))
}

#[utoipa::path(
    get,
    path = "/",
    summary = "Read one's own profile",
    description = "Returns the record of the user of the JWT, without having to know their \
                   id. It is the call the fronts make on load to show the \
                   name and the groups of the signed-in user.\n\n\
                   Same content as `GET /api/v1/user/{id}/`, minus the `is_archived` flag: an \
                   archived user cannot sign in.",
    responses(
        (
            status = 200,
            description = "Profile of the signed-in user.",
            body = GetMeResponseView,
            example = json!({
                "first_name": "Jean",
                "last_name": "Dupont",
                "email": "jean.dupont@mairie360.fr",
                "phone": "+33612345678",
                "phone_country": "FR",
                "status": "active",
                "role": "agent",
                "roles": ["agent"],
                "groups": [
                    { "id": 3, "owner_id": 2, "name": "Planning department", "description": "Building permit processing" }
                ]
            })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while reading the profile.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_me(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, GetMeError> {
    let me = trigger_get_me(state, auth_user.id).await?;
    Ok(HttpResponse::Ok().json(me))
}
