use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::database::sessions::{format_session_timestamp, Session};

/// Session d'un utilisateur (un appareil connecté).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionSchema {
    /// Identifiant de la session, sérialisé en chaîne bien qu'il soit numérique en base.
    #[schema(example = "1")]
    id: String,
    /// Description de l'appareil, telle que fournie par le client au login.
    #[schema(example = "Chrome 140 sur Windows 11")]
    device_info: String,
    /// Adresse IP depuis laquelle la session a été ouverte. Le rafraîchissement et la révocation
    /// exigent de repasser par cette même adresse.
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

impl From<Session> for SessionSchema {
    fn from(session: Session) -> Self {
        Self {
            id: session.id().to_string(),
            device_info: session.device_info().to_string(),
            ip_address: session.ip_address().to_string(),
            created_at: format_session_timestamp(session.created_at()),
            expires_at: format_session_timestamp(session.expires_at()),
            revoked_at: session.revoked_at().map(format_session_timestamp),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SessionSchema;
    use crate::database::sessions::Session;
    use chrono::{TimeZone, Utc};

    fn session(revoked: bool) -> Session {
        Session::new(
            uuid::Uuid::nil(),
            2,
            "Firefox".to_string(),
            "203.0.113.24".parse().unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 16, 8, 42, 11).unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 23, 8, 42, 11).unwrap(),
            revoked.then(|| Utc.with_ymd_and_hms(2026, 9, 18, 7, 15, 2).unwrap()),
        )
    }

    /// MAIR-767: `DateTime::to_string` gave `2026-09-16 08:42:11 UTC`, which the shared
    /// `SessionList` cannot parse; the session dates must be RFC 3339 / ISO 8601 UTC.
    #[test]
    fn mair_767_session_dates_are_iso_8601_utc() {
        let json = serde_json::to_value(SessionSchema::from(session(true))).unwrap();
        assert_eq!(json["created_at"], "2026-09-16T08:42:11Z");
        assert_eq!(json["expires_at"], "2026-09-23T08:42:11Z");
        assert_eq!(json["revoked_at"], "2026-09-18T07:15:02Z");
        for field in ["created_at", "expires_at", "revoked_at"] {
            let value = json[field].as_str().unwrap();
            assert!(
                chrono::DateTime::parse_from_rfc3339(value).is_ok(),
                "{field}: {value}"
            );
        }
    }

    #[test]
    fn mair_767_never_revoked_session_keeps_null_revoked_at() {
        let json = serde_json::to_value(SessionSchema::from(session(false))).unwrap();
        assert!(json["revoked_at"].is_null());
        assert_eq!(json["created_at"], "2026-09-16T08:42:11Z");
    }
}
