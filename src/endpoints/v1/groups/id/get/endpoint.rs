use crate::database::groups::does_group_exist::DoesGroupExistQuerView;
use crate::database::groups::get_group::GetGroupQuerView;
use crate::endpoints::v1::groups::id::get::view::GetGroupResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetGroupError {
    BadRequest,
    UnknowGroup,
}

impl std::fmt::Display for GetGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
            GetGroupError::UnknowGroup => {
                write!(f, "Unknow group.")
            }
        }
    }
}

impl ResponseError for GetGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetGroupError::BadRequest => StatusCode::BAD_REQUEST,
            GetGroupError::UnknowGroup => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_group(
    state: web::Data<AppState>,
    id: u64,
) -> Result<GetGroupResultView, GetGroupError> {
    let smart_db = state.get_smart_db();

    let group_check_view = DoesGroupExistQuerView::new(id);
    let result: bool = smart_db
        .fetch_scalar(&group_check_view)
        .await
        .map_err(|_| GetGroupError::UnknowGroup)?;
    if !result {
        return Err(GetGroupError::UnknowGroup);
    }

    let db_view = GetGroupQuerView::new(id);
    let result = smart_db
        .fetch_one(&db_view)
        .await
        .map_err(|_| GetGroupError::BadRequest)?;

    Ok(GetGroupResultView::new(result))
}

#[utoipa::path(
    get,
    path = "",
    summary = "Consulter un groupe",
    description = "Renvoie le nom, la description et le propriétaire d'un groupe. Accessible à \
                   tout utilisateur authentifié, y compris s'il n'est pas membre du groupe.\n\n\
                   Pour la liste de ses membres, voir `GET /api/v1/groups/{group_id}/users/`.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe.", example = 3)
    ),
    responses(
        (
            status = 200,
            description = "Le groupe demandé.",
            body = GetGroupResultView,
            example = json!({
                "group": { "id": 3, "owner_id": 2, "name": "Service urbanisme", "description": "Instruction des permis de construire" }
            })
        ),
        (
            status = 400,
            description = "Échec de la lecture du groupe une fois son existence confirmée. Ce endpoint renvoie `400` là où les autres renverraient `500`.",
            body = String,
            content_type = "text/plain",
            example = json!("Bad request.")
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
            example = json!("Unknow group.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
) -> Result<impl Responder, GetGroupError> {
    let result = trigger_get_group(state, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
