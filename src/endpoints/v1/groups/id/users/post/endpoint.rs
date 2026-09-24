use crate::database::groups::add_user_to_group::AddUserToGroupQueryView;
use crate::endpoints::v1::groups::id::users::post::view::PostUserGroupView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostUserGroupError {
    // BadRequest,
    UnknowUser,
}

impl std::fmt::Display for PostUserGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // PostUserGroupError::BadRequest => {
            //     write!(f, "Bad request.")
            // }
            Self::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for PostUserGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            // PostUserGroupError::BadRequest => StatusCode::BAD_REQUEST,
            Self::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_add_user_to_group(
    state: web::Data<AppState>,
    view: PostUserGroupView,
) -> Result<(), PostUserGroupError> {
    let db_view = AddUserToGroupQueryView::new(view.group_id(), view.user_id());
    state
        .get_smart_db()
        .execute(db_view)
        .await
        .map_err(|_| PostUserGroupError::UnknowUser)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    summary = "Ajouter un utilisateur à un groupe",
    description = "Rattache un utilisateur à un groupe. À partir de là, le groupe apparaît dans le \
                   `GET /api/v1/groups/` de cet utilisateur.\n\n\
                   Attention : le groupe visé est celui du champ `group_id` du **corps**, pas celui \
                   du chemin. Le `group_id` de l'URL est ignoré par le handler ; renseigner les \
                   deux avec la même valeur pour éviter toute ambiguïté.\n\n\
                   Cet endpoint ne vérifie pas non plus que l'appelant est propriétaire du groupe.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe. **Ignoré** : c'est le `group_id` du corps qui fait foi.", example = 3)
    ),
    request_body(
        content = PostUserGroupView,
        description = "Utilisateur à rattacher et groupe de destination.",
        example = json!({ "user_id": 42, "group_id": 3 })
    ),
    responses(
        (
            status = 200,
            description = "Utilisateur rattaché au groupe.",
            body = String,
            content_type = "text/plain",
            example = json!("User added to group successfully")
        ),
        (
            status = 400,
            description = "Corps JSON malformé ou champ obligatoire absent.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `user_id`")
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
            description = "`user_id` ou `group_id` ne correspond à rien, ou l'utilisateur est déjà membre du groupe.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow user.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn add_user_to_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    view: web::Json<PostUserGroupView>,
) -> Result<impl Responder, PostUserGroupError> {
    trigger_add_user_to_group(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("User added to group successfully"))
}
