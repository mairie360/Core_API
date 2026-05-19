use crate::database::groups::get_group::Group;
use crate::database::groups::get_user_groups::view::GetUserGroupsQuerView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_user_groups(
    view: GetUserGroupsQuerView,
    pool: PgPool,
) -> Result<Vec<Group>, DatabaseError> {
    let result: Vec<Group> = sqlx::query_as(&view.get_request())
        .bind(view.user_id() as i32)
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
