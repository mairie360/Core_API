use crate::{
    database::{
        admin::get_user::view::{
            AdminGetUserQueryResultView, AdminGetUserQueryView, RoleQueryResult, User,
        },
        groups::get_user_groups::GetUserGroupsQuerView,
        roles::get_roles_by_id::{GetRolesByIdQueryView, Role},
        sessions::{get_sessions_by_user::GetSessionsByUserQueryView, Session},
        users::get_roles::GetUserRolesQueryView,
    },
    endpoints::v1::admin::users::id::get::view::GetUserResultView,
};
use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    UnknownUser,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUserError::UnknownUser => write!(f, "Unknown user"),
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUserError::UnknownUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user(
    state: web::Data<AppState>,
    user_id: u64,
) -> Result<GetUserResultView, GetUserError> {
    let smart_db = state.get_smart_db();

    let user: User = smart_db
        .fetch_one(&AdminGetUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;

    let roles_id: Vec<i32> = smart_db
        .fetch_all(&GetUserRolesQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;
    let roles_result: Vec<Role> = smart_db
        .fetch_all(&GetRolesByIdQueryView::new(roles_id.clone()))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;
    let mut roles: Vec<RoleQueryResult> = Vec::new();
    for i in 0..roles_result.len() {
        roles.push(RoleQueryResult::new(
            roles_id[i],
            roles_result[i].name(),
            roles_result[i].description(),
        ));
    }

    let sessions: Vec<Session> = smart_db
        .fetch_all(&GetSessionsByUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;

    let groups = smart_db
        .fetch_all(&GetUserGroupsQuerView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            GetUserError::UnknownUser
        })?;

    let result = AdminGetUserQueryResultView::new(user, roles, groups, sessions);

    Ok(result.into())
}

#[utoipa::path(
    get,
    path = "",
    summary = "Consulter la fiche complète d'un utilisateur (administration)",
    description = "Renvoie tout ce que la plateforme sait d'un utilisateur : état civil, \
                   téléphone, statut, archivage, rôles, groupes et **historique complet de ses \
                   sessions** (révoquées et expirées comprises). Réservé aux administrateurs.\n\n\
                   C'est la vue la plus large sur un compte ; `GET /api/v1/user/{id}/`, ouverte à \
                   tous, ne renvoie ni les sessions ni le détail des rôles.\n\n\
                   Tout échec de lecture, y compris une panne de base, est rapporté en `404` : ce \
                   endpoint ne renvoie jamais `500`.",
    params(
        ("userId" = u64, Path, description = "Identifiant de l'utilisateur.", example = 42)
    ),
    responses(
        (
            status = 200,
            description = "Fiche complète de l'utilisateur.",
            body = GetUserResultView,
            example = json!({
                "user": {
                    "first_name": "Jean",
                    "last_name": "Dupont",
                    "email": "jean.dupont@mairie360.fr",
                    "phone_number": "0612345678",
                    "status": "active",
                    "is_archived": false
                },
                "roles": [{ "id": 2, "name": "agent", "description": "Agent municipal" }],
                "groups": [
                    { "id": 3, "owner_id": 2, "name": "Service urbanisme", "description": "Instruction des permis de construire" }
                ],
                "sessions": [
                    {
                        "id": "1",
                        "device_info": "Chrome 140 sur Windows 11",
                        "ip_address": "192.168.1.24",
                        "created_at": "2026-09-16 08:42:11 UTC",
                        "expires_at": "2026-09-23 08:42:11 UTC",
                        "revoked_at": null
                    }
                ]
            })
        ),
        (
            status = 400,
            description = "L'`userId` du chemin n'est pas un entier.",
            body = String,
            content_type = "text/plain",
            example = json!("can not parse \"abc\" to a u64")
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
            description = "Aucun utilisateur ne porte cet identifiant — ou la lecture de ses rôles, groupes ou sessions a échoué.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn admin_get_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, GetUserError> {
    let result = get_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
