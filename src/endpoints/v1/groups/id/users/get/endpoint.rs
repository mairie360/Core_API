use crate::database::groups::does_group_exist::DoesGroupExistQuerView;
use crate::database::groups::get_group_members::GetGroupUsersQueryView;
use crate::endpoints::v1::groups::id::users::get::view::GetGroupUsersResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUsersGroupError {
    BadRequest,
    UnknowGroup,
}

impl std::fmt::Display for GetUsersGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUsersGroupError::BadRequest => {
                write!(f, "Bad request")
            }
            GetUsersGroupError::UnknowGroup => {
                write!(f, "Unknow group")
            }
        }
    }
}

impl ResponseError for GetUsersGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUsersGroupError::BadRequest => StatusCode::BAD_REQUEST,
            GetUsersGroupError::UnknowGroup => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_group_members(
    state: web::Data<AppState>,
    group_id: i32,
) -> Result<GetGroupUsersResultView, GetUsersGroupError> {
    let smart_db = state.get_smart_db();

    let check_view = DoesGroupExistQuerView::new(group_id as u64);
    let result: bool = smart_db
        .fetch_scalar(&check_view)
        .await
        .map_err(|_| GetUsersGroupError::UnknowGroup)?;
    if !result {
        return Err(GetUsersGroupError::UnknowGroup);
    }

    let view = GetGroupUsersQueryView::new(group_id as u64);
    let result: Vec<i32> = smart_db
        .fetch_all(&view)
        .await
        .map_err(|_| GetUsersGroupError::BadRequest)?;

    Ok(result.into())
}

#[utoipa::path(
    get,
    path = "",
    summary = "Lister les membres d'un groupe",
    description = "Renvoie les identifiants des utilisateurs membres du groupe. Seuls les \
                   identifiants sont renvoyés : pour obtenir leurs noms et adresses, les repasser \
                   à `GET /api/v1/user/?ids=1,2,3`.\n\n\
                   La liste est vide si le groupe n'a aucun membre — ce qui est le cas juste après \
                   sa création, le propriétaire n'étant pas ajouté automatiquement.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe.", example = 3)
    ),
    responses(
        (
            status = 200,
            description = "Identifiants des membres du groupe.",
            body = GetGroupUsersResultView,
            example = json!({ "users": [1, 2, 5] })
        ),
        (
            status = 400,
            description = "Échec de la lecture des membres une fois le groupe trouvé. Ce endpoint renvoie `400` là où les autres renverraient `500`.",
            body = String,
            content_type = "text/plain",
            example = json!("Bad request")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "Aucun groupe ne porte cet identifiant.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow group")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_group_members(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    group_id: web::Path<i32>,
) -> Result<impl Responder, GetUsersGroupError> {
    let result = trigger_get_group_members(state, group_id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
