use crate::database::auth::register::RegisterUserQueryView;
use crate::endpoints::v1::admin::users::post::view::CreateUserView;
use crate::endpoints::validation::ValidatedJson;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum CreateUserError {
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserAlreadyExists => write!(f, "User already exists"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for CreateUserError {
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
    register_view: &CreateUserView,
    smart_db: &SmartDatabase,
) -> Result<(), CreateUserError> {
    let exists: bool = smart_db
        .fetch_scalar(&DoesUserExistByEmailQueryView::new(
            register_view.email().to_string(),
        ))
        .await
        .map_err(|_| CreateUserError::DatabaseError)?;

    if exists {
        return Err(CreateUserError::UserAlreadyExists);
    }

    Ok(())
}

async fn register_user(
    register_view: &CreateUserView,
    state: web::Data<AppState>,
) -> Result<(), CreateUserError> {
    can_be_registered(register_view, state.get_smart_db()).await?;

    let view = RegisterUserQueryView::new(
        register_view.first_name(),
        register_view.last_name(),
        register_view.email(),
        register_view.password(),
        register_view.phone_number(),
    );

    let success: bool = state
        .get_smart_db()
        .fetch_scalar(&view)
        .await
        .map_err(|e| {
            eprintln!("Database error: {e}");
            CreateUserError::DatabaseError
        })?;

    if success {
        Ok(())
    } else {
        Err(CreateUserError::DatabaseError)
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Create a user account (administration)",
    description = "Creates an account on behalf of an administrator, without the person having \
                   to sign up. Administrators only.\n\n\
                   Validations applied before anything is written, in this order (the first failing field \
                   is named in the `400` body):\n\
                   - `first_name`, `last_name`: 1 to 64 characters, not blank, no control \
                   character, no `<` or `>`;\n\
                   - `email`: a valid e-mail address (`local@domain.tld`), at most 320 characters;\n\
                   - `password`: 8 to 255 characters, no control character;\n\
                   - `phone_number`: optional; when present, 10 to 15 digits only (no space, `+` \
                   or separator).\n\n\
                   The password given here is temporary: the account is flagged as first \
                   connection, and the user's first `POST /api/v1/auth/login` answers `412` so \
                   they choose their own.",
    request_body(
        content = CreateUserView,
        description = "État civil, identifiants provisoires et téléphone facultatif du compte à créer.",
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
            description = "Compte créé, en attente du changement de mot de passe à la première connexion.",
            body = String,
            content_type = "text/plain",
            example = json!("User created successfully!")
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, or a field breaking the validation rules above; the body names the first invalid field.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `email`: must be a valid e-mail address")
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
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn admin_post_user(
    payload: ValidatedJson<CreateUserView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, CreateUserError> {
    let register_view = payload.into_inner();

    register_user(&register_view, state).await?;

    Ok(HttpResponse::Created().body("User created successfully!"))
}
