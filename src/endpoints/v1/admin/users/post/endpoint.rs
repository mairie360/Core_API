use crate::database::auth::register::RegisterUserQueryView;
use crate::endpoints::v1::admin::users::post::view::CreateUserView;
use crate::password::hash_password;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum CreateUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => write!(f, "Invalid data provided"),
            Self::UserAlreadyExists => write!(f, "User already exists"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for CreateUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidData => StatusCode::BAD_REQUEST,
            Self::UserAlreadyExists => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn is_valid_email(email: &str) -> bool {
    if email.is_empty() {
        return false;
    }
    email.find('@').is_some_and(|index| {
        let domain = &email[index + 1..];
        !domain.is_empty() && domain.contains('.')
    })
}

const fn is_valid_password(password: &str) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

fn is_valid_phone_number(phone_number: Option<&str>) -> bool {
    //Need to be more complex and based on requirements
    phone_number.is_none_or(|num| num.len() >= 10 && num.chars().all(|c| c.is_ascii_digit()))
}

async fn can_be_registered(
    register_view: &CreateUserView,
    smart_db: &SmartDatabase,
) -> Result<(), CreateUserError> {
    if !is_valid_email(register_view.email()) {
        return Err(CreateUserError::InvalidData);
    }

    let exists: bool = smart_db
        .fetch_scalar(&DoesUserExistByEmailQueryView::new(
            register_view.email().to_string(),
        ))
        .await
        .map_err(|_| CreateUserError::DatabaseError)?;

    if exists {
        return Err(CreateUserError::UserAlreadyExists);
    }

    if !is_valid_password(register_view.password()) {
        return Err(CreateUserError::InvalidData);
    }
    if !is_valid_phone_number(register_view.phone_number()) {
        return Err(CreateUserError::InvalidData);
    }
    Ok(())
}

async fn register_user(
    register_view: &CreateUserView,
    state: web::Data<AppState>,
) -> Result<(), CreateUserError> {
    can_be_registered(register_view, state.get_smart_db()).await?;

    let hashed_password = hash_password(register_view.password()).map_err(|e| {
        eprintln!("Password hashing error: {e}");
        CreateUserError::DatabaseError
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
    summary = "Créer un compte utilisateur (administration)",
    description = "Crée un compte au nom d'un administrateur, sans que la personne ait à \
                   s'inscrire. Réservé aux administrateurs.\n\n\
                   Mêmes règles de validation que `POST /api/v1/auth/register` : e-mail de la \
                   forme `locale@domaine.tld`, mot de passe d'au moins 8 caractères, téléphone \
                   facultatif d'au moins 10 chiffres. Toutes partagent le même `400`.\n\n\
                   Le mot de passe fourni ici est provisoire : le compte est marqué en première \
                   connexion, et le premier `POST /api/v1/auth/login` de l'utilisateur répondra \
                   `412` pour lui faire choisir le sien.",
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
            description = "Corps JSON malformé, ou e-mail, mot de passe ou numéro de téléphone ne respectant pas les règles ci-dessus.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid data provided")
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
    payload: web::Json<CreateUserView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, CreateUserError> {
    let register_view = payload.into_inner();

    register_user(&register_view, state).await?;

    Ok(HttpResponse::Created().body("User created successfully!"))
}
