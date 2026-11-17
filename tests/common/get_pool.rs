use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::isolated::{isolated_db, IsolatedDb};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// The database of the test running on this thread (MAIR-1002). libtest runs every test on a
    /// thread of its own and the tests run on one thread (`#[tokio::test]`, `#[actix_web::test]`),
    /// so the first helper a test calls creates its database and the end of the thread drops it.
    static OWN_DB: RefCell<Option<Rc<IsolatedDb>>> = const { RefCell::new(None) };
}

/// The database of the calling test: a copy of the seeded template, created on the first call and
/// dropped when the test ends. No test reads what another one writes, so no row needs a unique
/// marker to stay apart. Every helper below (`core_db_url`, `get_pool`, `get_raw_pool`'s URL) goes
/// through it, so the fixtures, the `AppState` and the query views of a test see the same rows.
pub async fn own_db() -> Rc<IsolatedDb> {
    if let Some(db) = OWN_DB.with(|cell| cell.borrow().clone()) {
        return db;
    }
    let db = Rc::new(isolated_db().await);
    OWN_DB.with(|cell| *cell.borrow_mut() = Some(db.clone()));
    db
}

/// Superuser URL of the database of the calling test, for the fixtures (`get_raw_pool`).
pub async fn own_db_url() -> String {
    own_db().await.url()
}

// Aucune des vues migrées ne déclare de `cache_key`, donc `SmartDatabase` ne touche jamais
// Redis dans ces tests : une URL non joignable suffit (les échecs Redis sont silencieux côté lib).
const TEST_REDIS_URL: &str = "redis://127.0.0.1:6379";

/// Postgres role Core connects with in every deployment (Database `api_roles.sql`, MAIR-114).
const CORE_DB_ROLE: &str = "core_api";

/// URL of the database of the calling test as [`CORE_DB_ROLE`] (MAIR-556): the application code
/// under test (`AppState`, query views) goes through it, so a query the role is not granted fails
/// here with `permission denied` instead of in staging. The fixtures keep the superuser URL
/// (`own_db_url`, see `get_raw_pool`).
pub async fn core_db_url() -> String {
    own_db().await.url_as(CORE_DB_ROLE)
}

/// `SmartDatabase` on the database of the calling test, connected as [`CORE_DB_ROLE`].
pub async fn get_pool() -> SmartDatabase {
    let db = Database::new(&core_db_url().await).await;
    let redis = Redis::new(TEST_REDIS_URL);

    SmartDatabase::new(db, redis)
}

/// URL of the database of one test (`isolated_db()`, MAIR-1002) as [`CORE_DB_ROLE`], like
/// [`core_db_url`] for the shared one.
pub fn core_db_url_for(db: &IsolatedDb) -> String {
    db.url_as(CORE_DB_ROLE)
}

/// `SmartDatabase` on the database of one test, connected as [`CORE_DB_ROLE`].
pub async fn get_pool_for(db: &IsolatedDb) -> SmartDatabase {
    let database = Database::new(&core_db_url_for(db)).await;
    let redis = Redis::new(TEST_REDIS_URL);

    SmartDatabase::new(database, redis)
}
