use crate::common::{get_pool, roles::setup_tests};
use core_api::database::roles::does_role_exist::DoesRoleExistQueryView;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_does_role_exist_true() {
    setup_tests().await;
    let pool = get_pool().await;

    let view = DoesRoleExistQueryView::new(1);
    println!("{view}");
    let result: bool = pool.fetch_scalar(&view).await.unwrap();

    assert!(result);
}

#[tokio::test]
#[serial]
async fn test_does_role_exist_false() {
    setup_tests().await;
    let pool = get_pool().await;

    let result: bool = pool
        .fetch_scalar(&DoesRoleExistQueryView::new(999))
        .await
        .unwrap();

    assert!(!result);
}
