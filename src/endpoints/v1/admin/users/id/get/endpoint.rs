use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error::{self, DbFailure};
use crate::{
    database::{
        admin::get_user::view::{
            AdminGetUserQueryResultView, AdminGetUserQueryView, RoleQueryResult, User,
        },
        groups::get_user_groups::GetUserGroupsQuerView,
        roles::{get_roles_by_id::Role, get_roles_of_user::GetRolesOfUserQueryView},
        sessions::{get_sessions_by_user::GetSessionsByUserQueryView, Session},
    },
    endpoints::v1::admin::users::id::get::view::GetUserResultView,
};
use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
        .map_err(|e| match db_error::log("admin get user", &e) {
            DbFailure::NotFound => GetUserError::UnknownUser,
            _ => GetUserError::DatabaseError,
        })?;
    let user = user.with_e164_phone();

    let roles_result: Vec<Role> = smart_db
        .fetch_all(&GetRolesOfUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            db_error::log("admin get user", &e);
            GetUserError::DatabaseError
        })?;
    // Each role carries its own id: no pairing of two lists by index (MAIR-390).
    let roles: Vec<RoleQueryResult> = roles_result
        .iter()
        .map(|role| RoleQueryResult::new(role.id(), role.name(), role.description()))
        .collect();

    let sessions: Vec<Session> = smart_db
        .fetch_all(&GetSessionsByUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            db_error::log("admin get user", &e);
            GetUserError::DatabaseError
        })?;

    let groups = smart_db
        .fetch_all(&GetUserGroupsQuerView::new(user_id))
        .await
        .map_err(|e| {
            db_error::log("admin get user", &e);
            GetUserError::DatabaseError
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
                   `sessions` holds the latest 100 sessions of the account, newest first \
                   (MAIR-425), and `groups` its first 100 groups by name.\n\n\
                   An unknown `userId` answers `404`; a database failure answers `500`.",
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
                    "phone_number": "+33612345678",
                    "phone_country": "FR",
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
                        "ip_address": "203.0.113.24",
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
            description = "No user has this id.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "The user, or its roles, groups or sessions, could not be read. The cause is logged by the server.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn admin_get_user(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, GetUserError> {
    let result = get_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
