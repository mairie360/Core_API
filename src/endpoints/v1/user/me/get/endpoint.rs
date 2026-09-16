use crate::database::groups::get_user_groups::GetUserGroupsQuerView;
use crate::database::roles::get_roles_by_id::GetRolesByIdQueryView;
use crate::database::users::get_roles::GetUserRolesQueryView;
use crate::database::users::get_user_by_id::GetUserByIdQueryView;
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
            GetMeError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetMeError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
            eprintln!("Login DB Error: {}", e);
            GetMeError::DatabaseError
        })?;
    let view = GetUserGroupsQuerView::new(user_id);
    let groups = smart_db.fetch_all(&view).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetMeError::DatabaseError
    })?;
    let role = GetUserRolesQueryView::new(user_id);
    let role_id: Vec<i32> = smart_db.fetch_all(&role).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetMeError::DatabaseError
    })?;
    let view = GetRolesByIdQueryView::new(role_id);
    let role: Vec<crate::database::roles::get_roles_by_id::Role> =
        smart_db.fetch_all(&view).await.map_err(|e| {
            eprintln!("Login DB Error: {}", e);
            GetMeError::DatabaseError
        })?;

    Ok(GetMeResponseView::new(
        result.first_name(),
        result.last_name(),
        result.email(),
        result.phone_number(),
        result.status(),
        role[0].name(),
        groups,
    ))
}

#[utoipa::path(
    get,
    path = "/",
    summary = "Consulter son propre profil",
    description = "Renvoie la fiche de l'utilisateur porté par le JWT, sans avoir à connaître son \
                   identifiant. C'est l'appel que font les fronts au chargement pour afficher le \
                   nom et les groupes de l'utilisateur connecté.\n\n\
                   Même contenu que `GET /api/v1/user/{id}/`, moins le drapeau `is_archived` : un \
                   utilisateur archivé ne peut pas se connecter.",
    responses(
        (
            status = 200,
            description = "Profil de l'utilisateur connecté.",
            body = GetMeResponseView,
            example = json!({
                "first_name": "Jean",
                "last_name": "Dupont",
                "email": "jean.dupont@mairie360.fr",
                "phone": "0612345678",
                "status": "active",
                "role": "agent",
                "groups": [
                    { "id": 3, "owner_id": 2, "name": "Service urbanisme", "description": "Instruction des permis de construire" }
                ]
            })
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la lecture du profil.",
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
