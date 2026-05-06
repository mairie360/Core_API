use core_api::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

use crate::common::{get_pool, roles::setup_tests};

#[tokio::test]
#[serial]
async fn test_get_roles() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let roles = get_roles_query(GetRolesQueryView {}, pool).await.unwrap();

    assert!(roles.len() >= 1);
}
