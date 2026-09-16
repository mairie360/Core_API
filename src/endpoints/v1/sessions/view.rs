use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::database::sessions::Session;

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

impl From<Session> for SessionSchema {
    fn from(session: Session) -> Self {
        Self {
            id: session.id().to_string(),
            device_info: session.device_info().to_string(),
            ip_address: session.ip_address().to_string(),
            created_at: session.created_at().to_string(),
            expires_at: session.expires_at().to_string(),
            revoked_at: session.revoked_at().map(std::string::ToString::to_string),
        }
    }
}
