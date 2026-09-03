use crate::database::admin::get_user::view::{
    AdminGetUserQueryResultView, AdminGetUserQueryView, RoleQueryResult, User,
};
use crate::database::groups::get_user_groups::{get_user_groups, GetUserGroupsQuerView};
use crate::database::roles::get_roles_by_id::{get_roles_by_id_query, GetRolesByIdQueryView, Role};
use crate::database::sessions::get_sessions_by_user::{
    get_sessions_by_user_query, GetSessionsByUserQueryView,
};
use crate::database::sessions::Session;
use crate::database::users::get_roles::{get_user_roles_query, GetUserRolesQueryView};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_user_query(
    view: AdminGetUserQueryView,
    smart_db: &SmartDatabase,
) -> Result<AdminGetUserQueryResultView, ApiLibError> {
    let user: User = smart_db.fetch_one(&view).await?;

    let roles_id_view = GetUserRolesQueryView::new(view.user_id());
    let roles_id: Vec<i32> = get_user_roles_query(roles_id_view, smart_db).await?;
    let roles = get_roles_by_id_query(GetRolesByIdQueryView::new(roles_id.clone()), smart_db);

    let sessions_view = GetSessionsByUserQueryView::new(view.user_id());
    let sessions = get_sessions_by_user_query(sessions_view, smart_db);

    let roles_result: Vec<Role> = roles.await?;
    let mut roles: Vec<RoleQueryResult> = Vec::new();
    for i in 0..roles_result.len() {
        roles.push(RoleQueryResult::new(
            roles_id[i],
            roles_result[i].name(),
            roles_result[i].description(),
        ));
    }
    let sessions: Vec<Session> = sessions.await?;
    let view = GetUserGroupsQuerView::new(view.user_id());
    let groups = get_user_groups(view, smart_db).await?;
    let result = AdminGetUserQueryResultView::new(user, roles, groups, sessions);

    Ok(result)
}
