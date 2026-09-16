use crate::database::users::list_directory::{DirectoryUser, ListDirectoryUsersQueryView};
use crate::endpoints::v1::user::get::view::{
    parse_id_list, DirectoryUsersQuery, DirectoryUsersResultView, MAX_DIRECTORY_LIMIT,
};
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum DirectoryError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DirectoryError::BadRequest => write!(f, "Bad request."),
            DirectoryError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for DirectoryError {
    fn status_code(&self) -> StatusCode {
        match self {
            DirectoryError::BadRequest => StatusCode::BAD_REQUEST,
            DirectoryError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_list_directory_users(
    state: web::Data<AppState>,
    query: DirectoryUsersQuery,
) -> Result<DirectoryUsersResultView, DirectoryError> {
    let ids = parse_id_list(query.ids()).ok_or(DirectoryError::BadRequest)?;
    let group_ids = parse_id_list(query.group_ids()).ok_or(DirectoryError::BadRequest)?;
    let limit = query.limit().unwrap_or(MAX_DIRECTORY_LIMIT);
    if limit == 0 || limit > MAX_DIRECTORY_LIMIT {
        return Err(DirectoryError::BadRequest);
    }

    let users: Vec<DirectoryUser> = state
        .get_smart_db()
        .fetch_all(&ListDirectoryUsersQueryView::new(
            query.search(),
            &ids,
            &group_ids,
            limit,
        ))
        .await
        .map_err(|error| {
            eprintln!("{:?}", error);
            DirectoryError::DatabaseError
        })?;

    Ok(DirectoryUsersResultView { users })
}

#[utoipa::path(
    get,
    path = "",
    summary = "Rechercher dans l'annuaire des utilisateurs",
    description = "Renvoie les utilisateurs non archivés, avec leurs rôles et les identifiants des \
                   groupes auxquels ils appartiennent. Sert à alimenter les sélecteurs \
                   d'utilisateurs des autres modules (assignation de tâche, invitation à un \
                   événement, ajout à une conversation).\n\n\
                   Les filtres se cumulent : un utilisateur doit satisfaire **tous** ceux qui sont \
                   fournis. Sans aucun filtre, les 1000 premiers utilisateurs non archivés sont \
                   renvoyés.\n\n\
                   Vue restreinte volontairement : ni téléphone, ni statut, ni date. Pour la fiche \
                   complète d'un utilisateur, voir `GET /api/v1/user/{id}/`.",
    params(DirectoryUsersQuery),
    responses(
        (
            status = 200,
            description = "Utilisateurs correspondant aux filtres. La liste est vide si aucun ne correspond.",
            body = DirectoryUsersResultView,
            example = json!({
                "users": [
                    {
                        "id": 1,
                        "first_name": "Jean",
                        "last_name": "Dupont",
                        "email": "jean.dupont@mairie360.fr",
                        "roles": ["agent"],
                        "group_ids": [3, 7]
                    },
                    {
                        "id": 2,
                        "first_name": "Amina",
                        "last_name": "Bensaïd",
                        "email": "amina.bensaid@mairie360.fr",
                        "roles": ["admin", "agent"],
                        "group_ids": []
                    }
                ]
            })
        ),
        (
            status = 400,
            description = "`ids` ou `group_ids` ne sont pas des listes d'entiers strictement positifs séparés par des virgules, ou `limit` est hors de l'intervalle 1–1000.",
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
            status = 500,
            description = "Erreur de base de données lors de la lecture de l'annuaire.",
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
pub async fn list_directory_users(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    query: web::Query<DirectoryUsersQuery>,
) -> Result<impl Responder, DirectoryError> {
    let result = trigger_list_directory_users(state, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
