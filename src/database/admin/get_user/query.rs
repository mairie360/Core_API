use crate::database::admin::get_user::view::{
    AdminGetUserQueryResultView, AdminGetUserQueryView, User,
};
use crate::database::roles::get_roles_by_id::{get_roles_by_id_query, GetRolesByIdQueryView, Role};
use crate::database::sessions::get_sessions_by_user::{
    get_sessions_by_user_query, GetSessionsByUserQueryView,
};
use crate::database::sessions::Session;
use crate::database::users::get_roles::{get_user_roles_query, GetUserRolesdQueryView};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_user_query(
    view: AdminGetUserQueryView,
    pool: PgPool,
) -> Result<AdminGetUserQueryResultView, DatabaseError> {
    let user = sqlx::query_as::<_, User>(&view.get_request())
        .bind(view.user_id() as i32)
        .fetch_one(&pool)
        .await?;

    let roles_id_view = GetUserRolesdQueryView::new(view.user_id());
    let roles_id: Vec<i32> = get_user_roles_query(roles_id_view, pool.clone()).await?;
    let roles = get_roles_by_id_query(GetRolesByIdQueryView::new(roles_id), pool.clone());

    let sessions_view = GetSessionsByUserQueryView::new(view.user_id());
    let sessions = get_sessions_by_user_query(sessions_view, pool.clone());

    let roles: Vec<Role> = roles.await?;
    let sessions: Vec<Session> = sessions.await?;

    let result = AdminGetUserQueryResultView::new(user, roles, sessions);

    Ok(result)
}
