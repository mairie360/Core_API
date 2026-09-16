use crate::database::roles::get_roles::GetRolesQueryView;
use crate::endpoints::v1::admin::roles::get::view::AdminGetRolesResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    DatabaseError,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_roles(state: web::Data<AppState>) -> Result<AdminGetRolesResultView, GetError> {
    let view = GetRolesQueryView::default();
    let result = state.get_smart_db().fetch_all(&view).await.map_err(|e| {
        eprintln!("Login DB Error: {}", e);
        GetError::DatabaseError
    })?;
    Ok(AdminGetRolesResultView::from(result))
}

#[utoipa::path(
    get,
    path = "/",
    summary = "Lister les rôles (administration)",
    description = "Renvoie tous les rôles de la plateforme. Même contenu que \
                   `GET /api/v1/roles/`, mais sous le préfixe d'administration : c'est le point \
                   d'entrée des écrans d'admin, à côté des opérations d'écriture voisines.\n\n\
                   Réservé aux administrateurs. Un rôle sans description renvoie une chaîne vide, \
                   jamais `null`.",
    responses(
        (
            status = 200,
            description = "Liste des rôles de la plateforme.",
            body = AdminGetRolesResultView,
            example = json!({
                "roles": [
                    { "id": 1, "name": "admin", "description": "Administrateur de la plateforme" },
                    { "id": 2, "name": "agent", "description": "Agent municipal" }
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
            status = 403,
            description = "L'utilisateur est authentifié mais n'est pas administrateur.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la lecture des rôles.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[get("/")]
pub async fn admin_get_role(state: web::Data<AppState>) -> Result<impl Responder, GetError> {
    let roles = get_roles(state).await?;
    Ok(HttpResponse::Ok().json(roles))
}
