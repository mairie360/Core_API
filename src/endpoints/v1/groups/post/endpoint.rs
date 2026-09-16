use crate::database::groups::create_group::CreateGroupQueryView;
use crate::endpoints::v1::groups::post::view::{PostGroupResultView, PostGroupView};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostGroupError {
    BadRequest,
}

impl std::fmt::Display for PostGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for PostGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            PostGroupError::BadRequest => StatusCode::BAD_REQUEST,
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
    let id: i32 = state
        .get_smart_db()
        .fetch_scalar(&db_view)
        .await
        .map_err(|_| PostGroupError::BadRequest)?;

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
            description = "Corps JSON malformé, champ obligatoire absent, ou échec de l'insertion en base. Ce endpoint renvoie `400` là où les autres renverraient `500`.",
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
    view: web::Json<PostGroupView>,
) -> Result<impl Responder, PostGroupError> {
    let result = create_group(user, state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
