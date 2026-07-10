use crate::database::groups::get_group_members::view::GetGroupUsersQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_group_members_query(
    view: GetGroupUsersQueryView,
    pool: PgPool,
) -> Result<Vec<i32>, DatabaseError> {
    let result: Vec<i32> = sqlx::query_scalar(&view.get_request())
        .bind(view.group_id() as i32)
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
