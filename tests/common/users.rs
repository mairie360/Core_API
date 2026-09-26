use core_api::database::auth::link_identity::LinkUserIdentityQueryView;
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::keycloak::migration::KEYCLOAK_PROVIDER;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::seed_password_hash;
use sqlx::PgPool;

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
            first_name,
            marker,
            &email,
            seed_password_hash(),
            None,
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

/// Links `user_id` to the Keycloak subject `subject` through the schema's `link_user_identity()`.
pub async fn link_identity(pool: &SmartDatabase, user_id: i32, subject: &str) {
    let _: i32 = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            user_id,
            KEYCLOAK_PROVIDER,
            subject,
        ))
        .await
        .unwrap();
}

/// `(provider, subject)` links of `user_id`, ordered by provider.
pub async fn user_identities(raw: &PgPool, user_id: i32) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT provider, subject FROM user_identities WHERE user_id = $1 ORDER BY provider",
    )
    .bind(user_id)
    .fetch_all(raw)
    .await
    .unwrap()
}

/// Archives `user_id` through the soft-delete view, as `DELETE /api/v1/admin/users/{id}` does.
pub async fn archive_user(raw: &PgPool, user_id: i32) {
    sqlx::query("DELETE FROM v_users_active WHERE id = $1")
        .bind(user_id)
        .execute(raw)
        .await
        .unwrap();
}
