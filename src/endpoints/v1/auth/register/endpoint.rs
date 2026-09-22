use crate::database::auth::register::RegisterUserQueryView;
use crate::password::hash_password;
use actix_web::{error::ResponseError, http::StatusCode, post, web, HttpResponse, Responder};
use mairie360_api_lib::database::query_views::DoesUserExistByEmailQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

use super::register_view::RegisterView;

#[derive(Debug, Clone, PartialEq)]
enum RegisterError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => write!(f, "Invalid data provided"),
            Self::UserAlreadyExists => write!(f, "User already exists"),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for RegisterError {
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
    register_view: &RegisterView,
    smart_db: &SmartDatabase,
) -> Result<(), RegisterError> {
    if !is_valid_email(register_view.email()) {
        return Err(RegisterError::InvalidData);
    }

    let exists: bool = smart_db
        .fetch_scalar(&DoesUserExistByEmailQueryView::new(
            register_view.email().to_string(),
        ))
        .await
        .map_err(|_| RegisterError::DatabaseError)?;

    if exists {
        return Err(RegisterError::UserAlreadyExists);
    }

    if !is_valid_password(register_view.password()) {
        return Err(RegisterError::InvalidData);
    }
    if !is_valid_phone_number(register_view.phone_number()) {
        return Err(RegisterError::InvalidData);
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
    summary = "Créer un compte utilisateur",
    description = "Crée un compte à partir d'une adresse e-mail unique. Route publique : le \
                   `JwtMiddleware` laisse passer tout ce qui est sous `/auth`.\n\n\
                   Validations appliquées avant l'écriture en base :\n\
                   - `email` : non vide et de la forme `locale@domaine.tld` ;\n\
                   - `password` : au moins 8 caractères ;\n\
                   - `phone_number` : optionnel, mais s'il est fourni, au moins 10 chiffres \
                   uniquement (pas d'espace, de `+` ni de séparateur).\n\n\
                   Toutes ces validations partagent le même `400` et le même message : la réponse \
                   ne dit pas laquelle a échoué.",
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
            description = "Corps JSON malformé, ou e-mail, mot de passe ou numéro de téléphone ne respectant pas les règles ci-dessus.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid data provided")
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
    payload: web::Json<RegisterView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, RegisterError> {
    let register_view = payload.into_inner();

    register_user(&register_view, state).await?;

    Ok(HttpResponse::Created().body("User registered successfully!"))
}
