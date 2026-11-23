mod get_pool; // Accès à ton pool
#[allow(unused_imports)] // `own_db` is used by the test binaries that include `common` only
pub use get_pool::{core_db_url, core_db_url_for, get_pool, get_pool_for, own_db, own_db_url};
pub mod fake_resend;
mod get_raw_pool;
pub use get_raw_pool::get_raw_pool;
pub mod acl_redis;
pub mod roles;
pub mod users;

/// URL of the Redis shared by the test binary (one container): every sign-in and refresh publishes its
/// JWT there and fails without it (MAIR-772). Keys of different tests never meet (random session ids).
pub async fn redis_url() -> String {
    mairie360_api_lib::test_setup::redis_setup::get_shared_redis()
        .await
        .url
        .clone()
}
