use crate::database::{
    admin::get_user::view::{AdminGetUserQueryResultView, RoleQueryResult, User},
    groups::get_group::Group,
    sessions::{format_session_timestamp, Session},
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
    #[schema(example = "203.0.113.24")]
    ip_address: String,
    /// Session opening date, RFC 3339 / ISO 8601 in UTC (`AAAA-MM-JJTHH:MM:SSZ`).
    #[schema(example = "2026-09-16T08:42:11Z")]
    created_at: String,
    /// Date after which the refresh token is no longer accepted, RFC 3339 / ISO 8601 in UTC.
    #[schema(example = "2026-09-23T08:42:11Z")]
    expires_at: String,
    /// Revocation date, RFC 3339 / ISO 8601 in UTC, or `null` if the session was never revoked.
    #[schema(example = json!(null))]
    revoked_at: Option<String>,
}

impl From<&Session> for SessionResultView {
    fn from(value: &Session) -> Self {
        Self {
            id: value.id().to_string(),
            device_info: value.device_info().to_string(),
            ip_address: value.ip_address().to_string(),
            created_at: format_session_timestamp(value.created_at()),
            expires_at: format_session_timestamp(value.expires_at()),
            revoked_at: value.revoked_at().map(format_session_timestamp),
        }
    }
}

/// Rôle porté par l'utilisateur consulté.
#[derive(Deserialize, ToSchema, Serialize, Debug, Clone, PartialEq, Eq)]
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
            description: value.description().map(std::string::ToString::to_string),
        }
    }
}

/// Fiche complète d'un utilisateur pour l'administration : profil, rôles, groupes et sessions.
#[derive(Deserialize, ToSchema, Serialize, Debug, PartialEq, Eq)]
pub struct GetUserResultView {
    /// État civil et statut du compte.
    user: User,
    /// The user's role: a user holds one role (MAIR-564). `null` when the user has none.
    #[schema(required = true)]
    role: Option<RoleResultView>,
    /// Deprecated: `role` as a list, kept for the clients written when a user could hold several
    /// roles. At most one entry, empty when the user has none. Read `role` instead.
    #[schema(deprecated)]
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
        let roles: Vec<RoleResultView> =
            value.roles().iter().map(std::convert::Into::into).collect();
        Self {
            user: value.user().clone(),
            role: roles.first().cloned(),
            roles,
            groups: value.groups().into_iter().collect(),
            sessions: value
                .sessions()
                .iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

#[cfg(test)]
mod mair_767_tests {
    use super::SessionResultView;
    use crate::database::sessions::Session;
    use chrono::{TimeZone, Utc};

    /// MAIR-767: the administration user record lists sessions with ISO 8601 UTC dates too.
    #[test]
    fn mair_767_admin_session_dates_are_iso_8601_utc() {
        let session = Session::new(
            uuid::Uuid::nil(),
            2,
            "Firefox".to_string(),
            "203.0.113.24".parse().unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 16, 8, 42, 11).unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 23, 8, 42, 11).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 9, 18, 7, 15, 2).unwrap()),
        );
        let json = serde_json::to_value(SessionResultView::from(&session)).unwrap();
        assert_eq!(json["created_at"], "2026-09-16T08:42:11Z");
        assert_eq!(json["expires_at"], "2026-09-23T08:42:11Z");
        assert_eq!(json["revoked_at"], "2026-09-18T07:15:02Z");
    }
}
