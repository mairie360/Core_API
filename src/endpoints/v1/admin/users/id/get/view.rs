use crate::database::{
    admin::get_user::view::{AdminGetUserQueryResultView, RoleQueryResult, User},
    groups::get_group::Group,
    sessions::Session,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Session d'un utilisateur, telle que vue par un administrateur.
#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
struct SessionResultView {
    /// Identifiant de la session, sérialisé en chaîne bien qu'il soit numérique en base.
    #[schema(example = "1")]
    id: String,
    /// Description de l'appareil, telle que fournie par le client au login.
    #[schema(example = "Chrome 140 sur Windows 11")]
    device_info: String,
    /// Adresse IP depuis laquelle la session a été ouverte.
    #[schema(example = "192.168.1.24")]
    ip_address: String,
    /// Date d'ouverture de la session, au format `AAAA-MM-JJ HH:MM:SS UTC`.
    #[schema(example = "2026-09-16 08:42:11 UTC")]
    created_at: String,
    /// Date au-delà de laquelle le jeton de rafraîchissement n'est plus accepté.
    #[schema(example = "2026-09-23 08:42:11 UTC")]
    expires_at: String,
    /// Date de révocation, ou `null` si la session n'a jamais été révoquée.
    #[schema(example = json!(null))]
    revoked_at: Option<String>,
}

impl From<&Session> for SessionResultView {
    fn from(value: &Session) -> Self {
        Self {
            id: value.id().to_string(),
            device_info: value.device_info().to_string(),
            ip_address: value.ip_address().to_string(),
            created_at: value.created_at().to_string(),
            expires_at: value.expires_at().to_string(),
            revoked_at: value.revoked_at().map(|t| t.to_string()),
        }
    }
}

/// Rôle porté par l'utilisateur consulté.
#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
struct RoleResultView {
    /// Identifiant du rôle.
    #[schema(example = 2)]
    id: i32,
    /// Nom technique du rôle.
    #[schema(example = "agent")]
    name: String,
    /// Description du rôle, ou `null` s'il n'en a pas.
    #[schema(example = "Agent municipal")]
    description: Option<String>,
}

impl From<&RoleQueryResult> for RoleResultView {
    fn from(value: &RoleQueryResult) -> Self {
        Self {
            id: value.id(),
            name: value.name().to_string(),
            description: value.description().map(|d| d.to_string()),
        }
    }
}

/// Fiche complète d'un utilisateur pour l'administration : profil, rôles, groupes et sessions.
#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
pub struct GetUserResultView {
    /// État civil et statut du compte.
    user: User,
    /// Rôles portés par l'utilisateur. Vide s'il n'en a aucun.
    roles: Vec<RoleResultView>,
    /// Groupes dont l'utilisateur est membre. Vide s'il n'appartient à aucun.
    groups: Vec<Group>,
    /// Historique complet des sessions, révoquées et expirées comprises.
    sessions: Vec<SessionResultView>,
}

impl Display for GetUserResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetUserResultView {{ user: {}, roles: {}, groups: {}, sessions: {} }}",
            self.user,
            self.roles.len(),
            self.groups.len(),
            self.sessions.len()
        )
    }
}

impl From<AdminGetUserQueryResultView> for GetUserResultView {
    fn from(value: AdminGetUserQueryResultView) -> Self {
        Self {
            user: value.user().clone(),
            roles: value.roles().iter().map(|r| r.into()).collect(),
            groups: value.groups().into_iter().collect(),
            sessions: value.sessions().iter().map(|s| s.into()).collect(),
        }
    }
}
