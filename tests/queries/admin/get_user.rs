use crate::common::get_pool;
use core_api::database::admin::get_user::view::{
    AdminGetUserQueryResultView, AdminGetUserQueryView, RoleQueryResult, User,
};
use core_api::database::groups::get_user_groups::GetUserGroupsQuerView;
use core_api::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use core_api::database::sessions::{get_sessions_by_user::GetSessionsByUserQueryView, Session};
use core_api::database::users::get_roles::GetUserRolesQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

// Reprend l'assemblage que fait `admin/users/id/get/endpoint.rs`, pour exercer les vues
// composées (`AdminGetUserQueryResultView` et consorts) au niveau de la couche base de données.
async fn get_user(
    smart_db: &SmartDatabase,
    user_id: u64,
) -> Result<AdminGetUserQueryResultView, ApiLibError> {
    let user: User = smart_db
        .fetch_one(&AdminGetUserQueryView::new(user_id))
        .await?;

    let roles_id: Vec<i32> = smart_db
        .fetch_all(&GetUserRolesQueryView::new(user_id))
        .await?;
    let roles_result: Vec<Role> = smart_db
        .fetch_all(&GetRolesByIdQueryView::new(roles_id.clone()))
        .await?;
    let mut roles: Vec<RoleQueryResult> = Vec::new();
    for i in 0..roles_result.len() {
        roles.push(RoleQueryResult::new(
            roles_id[i],
            roles_result[i].name(),
            roles_result[i].description(),
        ));
    }

    let sessions: Vec<Session> = smart_db
        .fetch_all(&GetSessionsByUserQueryView::new(user_id))
        .await?;

    let groups = smart_db
        .fetch_all(&GetUserGroupsQuerView::new(user_id))
        .await?;

    Ok(AdminGetUserQueryResultView::new(
        user, roles, groups, sessions,
    ))
}

#[tokio::test]
#[serial]
async fn get_user_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = get_user(&pool, *ALICE_ID.get().unwrap() as u64).await;

    assert!(result.is_ok(), "{:?}", result);
    let result = result.unwrap();
    assert!(format!("{}", result.user()).contains("Alice"));
    println!("{}", result);

    let roles = result.roles();
    assert!(!roles.is_empty());
    let role = &roles[0];
    println!("{:?}", role);
    assert!(!role.name().is_empty());
    let _ = role.description();

    let _ = result.groups();
    let sessions = result.sessions();
    assert!(!sessions.is_empty());
}

#[tokio::test]
#[serial]
async fn get_user_bad_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = get_user(&pool, 999_999).await;

    assert!(result.is_err(), "{:?}", result);
}
