use core_api::database::roles::get_roles::{GetRolesQueryView, RoleQueryResult};
use serial_test::serial;

use crate::common::{get_pool, roles::setup_tests};

#[tokio::test]
#[serial]
async fn test_get_roles() {
    setup_tests().await;
    let pool = get_pool().await;

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

    assert!(!roles.is_empty());
    let role = &roles[0];
    println!("{role}");
    let _ = role.created_at();
}
