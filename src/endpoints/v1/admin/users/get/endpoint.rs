use crate::database::admin::list_users::{
    AdminCountUsersQueryView, AdminListUsersQueryView, AdminUserRow,
};
use crate::endpoints::v1::admin::users::get::view::{
    AdminListUsersQuery, AdminListUsersResultView, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};
use crate::endpoints::validation::ValidatedQuery;
use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum ListUsersError {
    InvalidPagination,
    DatabaseError,
}

impl std::fmt::Display for ListUsersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListUsersError::InvalidPagination => write!(f, "Invalid pagination"),
            ListUsersError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for ListUsersError {
    fn status_code(&self) -> StatusCode {
        match self {
            ListUsersError::InvalidPagination => StatusCode::BAD_REQUEST,
            ListUsersError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn list_users(
    state: web::Data<AppState>,
    query: AdminListUsersQuery,
) -> Result<AdminListUsersResultView, ListUsersError> {
    let page = query.page().unwrap_or(1);
    let page_size = query.page_size().unwrap_or(DEFAULT_PAGE_SIZE);
    if page == 0 || page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(ListUsersError::InvalidPagination);
    }

    let smart_db = state.get_smart_db();
    let users_view =
        AdminListUsersQueryView::new(query.search(), query.group_id(), page, page_size);
    let count_view = AdminCountUsersQueryView::new(query.search(), query.group_id());
    let (users, total) = futures_util::try_join!(
        smart_db.fetch_all::<AdminUserRow, _>(&users_view),
        smart_db.fetch_scalar::<i64, _>(&count_view),
    )
    .map_err(|error| {
        eprintln!("{:?}", error);
        ListUsersError::DatabaseError
    })?;

    let total = total.max(0) as u64;
    Ok(AdminListUsersResultView {
        users,
        page,
        page_size,
        total,
        total_pages: total.div_ceil(page_size),
    })
}

#[utoipa::path(
    get,
    path = "",
    summary = "Lister les utilisateurs (administration)",
    description = "Vue d'administration paginée sur les comptes utilisateurs, archivés compris, \
                   avec téléphone, statut et rôles. Réservé aux administrateurs.\n\n\
                   À distinguer de `GET /api/v1/user/`, qui est l'annuaire ouvert à tous : celui-ci \
                   masque les comptes archivés, ne pagine pas et ne renvoie ni téléphone ni statut.\n\n\
                   `total` compte les utilisateurs correspondant aux filtres, toutes pages \
                   confondues ; `total_pages` en découle. Une `page` au-delà de `total_pages` \
                   renvoie une liste vide, pas une erreur.",
    params(AdminListUsersQuery),
    responses(
        (
            status = 200,
            description = "Page d'utilisateurs correspondant aux filtres.",
            body = AdminListUsersResultView,
            example = json!({
                "users": [
                    {
                        "id": 42,
                        "first_name": "Jean",
                        "last_name": "Dupont",
                        "email": "jean.dupont@mairie360.fr",
                        "phone_number": "0612345678",
                        "status": "active",
                        "is_archived": false,
                        "roles": [{ "id": 2, "name": "agent" }]
                    }
                ],
                "page": 1,
                "page_size": 20,
                "total": 137,
                "total_pages": 7
            })
        ),
        (
            status = 400,
            description = "`page` is 0, `page_size` is 0 or above 500, or `search` is longer than 255 characters or contains a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid pagination")
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
            status = 500,
            description = "Erreur de base de données lors de la lecture ou du comptage.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn admin_list_users(
    state: web::Data<AppState>,
    query: ValidatedQuery<AdminListUsersQuery>,
) -> Result<impl Responder, ListUsersError> {
    let result = list_users(state, query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
