use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use mairie360_api_lib::smart_db::SmartDatabase;

/// Crée un utilisateur dont le nom de famille et l'email contiennent `marker`, et renvoie son id.
pub async fn create_user(pool: &SmartDatabase, first_name: &str, marker: &str) -> i32 {
    let email = format!(
        "{}.{}.{}@example.com",
        first_name.to_lowercase(),
        marker,
        uuid::Uuid::new_v4()
    );
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            first_name, marker, &email, "password", None,
        ))
        .await
        .unwrap();
    pool.fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap()
}

/// Marqueur unique (lettres uniquement) pour isoler les données d'un test dans la base partagée.
pub fn unique_marker(prefix: &str) -> String {
    let suffix: String = uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .take(8)
        .collect();
    format!("{prefix}{suffix}")
}
