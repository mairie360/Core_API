use core_api::database::roles::get_roles::{GetRolesQueryView, RoleQueryResult};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

use crate::common::{get_pool, roles::setup_tests};

#[tokio::test]
#[serial]
async fn test_get_roles() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

    assert!(!roles.is_empty());
    let role = &roles[0];
    println!("{}", role);
    let _ = role.created_at();
}
