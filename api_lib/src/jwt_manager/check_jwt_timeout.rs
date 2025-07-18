use std::time::{SystemTime, UNIX_EPOCH};
use super::get_jwt_timeout::get_jwt_timeout;

pub fn verify_jwt_timeout(jwt_expiration: usize) -> Result<bool, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("SystemTime error: {:?}", e))?
        .as_secs() as usize;

    let timeout = get_jwt_timeout()?;

    if jwt_expiration < now {
        // Token déjà expiré
        Ok(false)
    } else {
        let diff = jwt_expiration - now;
        // On accepte une petite marge d'erreur (exemple 5 secondes)
        let margin = 5;
        Ok(diff + margin >= timeout)
    }
}
