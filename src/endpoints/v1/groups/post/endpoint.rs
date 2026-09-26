use crate::database::groups::create_group::CreateGroupQueryView;
use crate::endpoints::v1::groups::post::view::{PostGroupResultView, PostGroupView};
use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostGroupError {
    BadRequest,
    Duplicate,
}

impl std::fmt::Display for PostGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => {
                write!(f, "Bad request.")
            }
            Self::Duplicate => {
                write!(f, "A group with this name already exists.")
            }
        }
    }
}

impl ResponseError for PostGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Duplicate => StatusCode::CONFLICT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn create_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: PostGroupView,
) -> Result<PostGroupResultView, PostGroupError> {
    let db_view = CreateGroupQueryView::new(user.id, view.name(), view.description());
    let id: i32 =
        state
            .get_smart_db()
            .fetch_scalar(&db_view)
            .await
            .map_err(|error| match error {
                // `groups.name` is UNIQUE.
                ApiLibError::Database(DbError::UniqueViolation(_)) => PostGroupError::Duplicate,
                _ => PostGroupError::BadRequest,
            })?;

    Ok(PostGroupResultView::new(id as u64))
}

#[utoipa::path(
    post,
    path = "",
    summary = "Créer un groupe",
    description = "Crée un groupe dont l'utilisateur porté par le JWT devient le propriétaire \
                   (`owner_id`). Il n'en est pas membre pour autant : il faut l'ajouter \
                   explicitement via `POST /api/v1/groups/{group_id}/users/` pour qu'il \
                   apparaisse dans `GET /api/v1/groups/`.\n\n\
                   La réponse ne contient que l'identifiant attribué ; relire le groupe complet \
                   avec `GET /api/v1/groups/{group_id}/`.",
    request_body(
        content = PostGroupView,
        description = "Nom et description du groupe. Les deux champs sont obligatoires ; passer une chaîne vide pour une description absente.",
        example = json!({
            "name": "Service urbanisme",
            "description": "Instruction des permis de construire"
        })
    ),
    responses(
        (
            status = 200,
            description = "Groupe créé. Le corps contient l'identifiant attribué.",
            body = PostGroupResultView,
            example = json!({ "id": 3 })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, a field breaking its rules (`name` 1 to 64 characters once trimmed, no control character, no `<` or `>`; `description` at most 1000 characters, no `<` or `>`, no control character other than line breaks and tabs), or failed insert. This endpoint answers `400` where others would answer `500`.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `name`: must be at most 64 characters")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 409,
            description = "`name` is already used by another group (group names are unique).",
            body = String,
            content_type = "text/plain",
            example = json!("A group with this name already exists.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn post_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: ValidatedJson<PostGroupView>,
) -> Result<impl Responder, PostGroupError> {
    let result = create_group(user, state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
