use crate::database::auth::register::RegisterUserQueryView;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

use super::register_view::RegisterView;
use crate::endpoints::validation::ValidatedJson;

#[derive(Debug, Clone, PartialEq)]
enum RegisterError {
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserAlreadyExists => write!(f, "User already exists"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for RegisterError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UserAlreadyExists => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn can_be_registered(
    register_view: &RegisterView,
    smart_db: &SmartDatabase,
) -> Result<(), RegisterError> {
    let exists: bool = smart_db
        .fetch_scalar(&DoesUserExistByEmailQueryView::new(
            register_view.email().to_string(),
        ))
        .await
        .map_err(|_| RegisterError::DatabaseError)?;

    if exists {
        return Err(RegisterError::UserAlreadyExists);
    }

    Ok(())
}

async fn register_user(
    register_view: &RegisterView,
    state: web::Data<AppState>,
) -> Result<(), RegisterError> {
    can_be_registered(register_view, state.get_smart_db()).await?;

    let hashed_password = hash_password(register_view.password()).map_err(|e| {
        eprintln!("Password hashing error: {e}");
        RegisterError::DatabaseError
    })?;

    let view = RegisterUserQueryView::new(
        register_view.first_name(),
        register_view.last_name(),
        register_view.email(),
        &hashed_password,
        register_view.phone_number(),
    );

    let success: bool = state
        .get_smart_db()
        .fetch_scalar(&view)
        .await
        .map_err(|e| {
            eprintln!("Database error: {e}");
            RegisterError::DatabaseError
        })?;

    if success {
        Ok(())
    } else {
        Err(RegisterError::DatabaseError)
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Create a user account",
    description = "Creates an account for a unique e-mail address. Public route: the \
                   `JwtMiddleware` lets everything under `/auth` through.\n\n\
                   Validations applied before anything is written, in this order (the first failing field \
                   is named in the `400` body):\n\
                   - `first_name`, `last_name`: 1 to 64 characters, not blank, no control \
                   character, no `<` or `>`;\n\
                   - `email`: a valid e-mail address (`local@domain.tld`), at most 320 characters;\n\
                   - `password`: 8 to 255 characters, no control character;\n\
                   - `phone_number`: optional; when present, 10 to 15 digits only (no space, `+` \
                   or separator).",
    request_body(
        content = RegisterView,
        description = "État civil, identifiants et téléphone facultatif du nouvel utilisateur.",
        example = json!({
            "first_name": "Jean",
            "last_name": "Dupont",
            "email": "jean.dupont@mairie360.fr",
            "password": "MotDePasse!123",
            "phone_number": "0612345678"
        })
    ),
    responses(
        (
            status = 201,
            description = "Compte créé. L'utilisateur est marqué en première connexion : son premier \
                           `POST /api/v1/auth/login` répondra `412` et exigera un changement de mot de passe.",
            body = String,
            content_type = "text/plain",
            example = json!("User registered successfully!")
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, or a field breaking the validation rules above; the body names the first invalid field.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `email`: must be a valid e-mail address")
        ),
        (
            status = 409,
            description = "Un compte utilise déjà cette adresse e-mail.",
            body = String,
            content_type = "text/plain",
            example = json!("User already exists")
        ),
        (
            status = 500,
            description = "Erreur de base de données pendant la vérification d'unicité ou l'insertion.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Auth"
)]
#[post("/register")]
pub async fn register(
    payload: ValidatedJson<RegisterView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, RegisterError> {
    let register_view = payload.into_inner();

    register_user(&register_view, state).await?;

    Ok(HttpResponse::Created().body("User registered successfully!"))
}
