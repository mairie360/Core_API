use crate::database::users::delete_user::DeleteUserQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn delete_user_query(
    view: DeleteUserQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.user_id() as i32)
        .execute(&pool)
        .await?;

    Ok(())
}
