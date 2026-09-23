use crate::database::groups::get_group::Group;
use crate::database::groups::update_group::UpdateGroupQueryView;
use crate::endpoints::v1::groups::id::patch::view::{PatchGroupView, MAX_GROUP_NAME_LENGTH};
use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchGroupError {
    BadRequest,
    UnknownGroup,
    DatabaseError,
}

impl std::fmt::Display for PatchGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => write!(f, "Bad request."),
            Self::UnknownGroup => write!(f, "Unknow group"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for PatchGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::UnknownGroup => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_patch_group(
    state: web::Data<AppState>,
    group_id: u64,
    view: PatchGroupView,
) -> Result<Group, PatchGroupError> {
    let name = view.name().map(str::trim);
    if view.name().is_none() && view.description().is_none() {
        return Err(PatchGroupError::BadRequest);
    }
    if name.is_some_and(|name| name.is_empty() || name.chars().count() > MAX_GROUP_NAME_LENGTH) {
        return Err(PatchGroupError::BadRequest);
    }

    let groups: Vec<Group> = state
        .get_smart_db()
        .fetch_all(&UpdateGroupQueryView::new(
            group_id,
            name,
            view.description(),
        ))
        .await
        .map_err(|error| {
            eprintln!("{error:?}");
            PatchGroupError::DatabaseError
        })?;

    groups
        .into_iter()
        .next()
        .ok_or(PatchGroupError::UnknownGroup)
}

#[utoipa::path(
    patch,
    path = "",
    summary = "Modifier un groupe",
    description = "Met à jour le nom ou la description d'un groupe et renvoie le groupe tel \
                   qu'enregistré. Modification partielle : un champ absent ou `null` reste \
                   inchangé.\n\n\
                   Au moins un des deux champs doit être fourni : un corps vide est refusé en \
                   `400`. Le nom, une fois nettoyé de ses espaces de bord, ne peut être ni vide ni \
                   plus long que 255 caractères.\n\n\
                   Attention : cet endpoint ne vérifie pas que l'appelant est propriétaire du \
                   groupe. Tout utilisateur authentifié peut modifier n'importe quel groupe.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe.", example = 3)
    ),
    request_body(
        content = PatchGroupView,
        description = "Champs à modifier. Les deux sont facultatifs, mais au moins un doit être présent.",
        example = json!({ "description": "Instruction des permis de construire et des déclarations préalables" })
    ),
    responses(
        (
            status = 200,
            description = "Groupe mis à jour.",
            body = Group,
            example = json!({
                "id": 3,
                "owner_id": 2,
                "name": "Service urbanisme",
                "description": "Instruction des permis de construire et des déclarations préalables"
            })
        ),
        (
            status = 400,
            description = "Corps JSON malformé, aucun champ à modifier, ou nom vide ou de plus de 255 caractères.",
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
            status = 404,
            description = "Aucun groupe ne porte cet identifiant.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow group")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la mise à jour.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn patch_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
    view: web::Json<PatchGroupView>,
) -> Result<impl Responder, PatchGroupError> {
    let group = trigger_patch_group(state, id.into_inner(), view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(group))
}
