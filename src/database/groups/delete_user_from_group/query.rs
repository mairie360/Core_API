use crate::database::groups::delete_user_from_group::DeleteUserFromGroupQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn delete_user_from_group_query(
    view: DeleteUserFromGroupQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.group_id() as i32)
        .bind(view.user_id() as i32)
        .execute(&pool)
        .await?;

    Ok(())
}
