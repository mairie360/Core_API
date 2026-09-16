use super::admin::doc::AdminDoc;
use super::auth::doc::AuthDoc;
use super::groups::doc::GroupsDoc;
use super::roles::doc::RolesDoc;
use super::sessions::doc::SessionsDoc;
use super::user::doc::UserDoc;
use utoipa::OpenApi;

// `ressources` n'est pas monté dans `config` (add_access n'a pas de contrôle d'autorisation) : ses
// opérations ne sont pas publiées tant que les routes ne sont pas réellement servies.
#[derive(OpenApi)]
#[openapi(nest(
    (path = "/admin", api = AdminDoc),
    (path = "/auth", api = AuthDoc),
    (path = "/groups", api = GroupsDoc),
    (path = "/roles", api = RolesDoc),
    (path = "/sessions", api = SessionsDoc),
    (path = "/user", api = UserDoc),
))]
pub struct V1Doc;
