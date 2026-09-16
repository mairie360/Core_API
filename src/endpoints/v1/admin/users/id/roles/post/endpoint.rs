use crate::database::users::add_role::AddRolesQueryView;
use crate::endpoints::v1::admin::users::id::roles::post::view::AddRoleToUserView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum AddRoleToUserError {
    NotFound,
}

impl std::fmt::Display for AddRoleToUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddRoleToUserError::NotFound => {
                write!(f, "User or role not found.")
            }
        }
    }
}

impl ResponseError for AddRoleToUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            AddRoleToUserError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn add_role_to_user(
    state: web::Data<AppState>,
    view: AddRoleToUserView,
) -> Result<(), AddRoleToUserError> {
    let view = AddRolesQueryView::new(view.role_id(), view.user_id());
    state
        .get_smart_db()
        .execute(view)
        .await
        .map_err(|_| AddRoleToUserError::NotFound)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/",
    summary = "Attribuer un rôle à un utilisateur",
    description = "Rattache un rôle à un compte utilisateur. Réservé aux administrateurs.\n\n\
                   Attention : l'utilisateur visé est celui du champ `user_id` du **corps**, pas \
                   celui du chemin. L'`userId` de l'URL est ignoré par le handler ; renseigner les \
                   deux avec la même valeur pour éviter toute ambiguïté.\n\n\
                   La réponse a un corps vide. Tout échec d'écriture est rapporté en `404` : ce \
                   endpoint ne renvoie jamais `500`.",
    request_body(
        content = AddRoleToUserView,
        description = "Rôle à attribuer et utilisateur qui le reçoit.",
        example = json!({ "role_id": 2, "user_id": 42 })
    ),
    params(
        ("userId" = u64, Path, description = "Identifiant de l'utilisateur. **Ignoré** : c'est le `user_id` du corps qui fait foi.", example = 42)
    ),
    responses(
        (
            status = 200,
            description = "Rôle attribué. Corps vide.",
        ),
        (
            status = 400,
            description = "Corps JSON malformé ou champ obligatoire absent.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `role_id`")
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "L'utilisateur est authentifié mais n'est pas administrateur.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 404,
            description = "`user_id` ou `role_id` ne correspond à rien, ou l'utilisateur porte déjà ce rôle.",
            body = String,
            content_type = "text/plain",
            example = json!("User or role not found.")
        ),
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Users"
)]
#[post("/")]
pub async fn admin_add_role_to_user(
    state: web::Data<AppState>,
    view: web::Json<AddRoleToUserView>,
) -> Result<impl Responder, AddRoleToUserError> {
    add_role_to_user(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
