use crate::database::users::remove_role::RemoveRolesQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn remove_role_query(
    view: RemoveRolesQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.role_id() as i32)
        .bind(view.user_id() as i32)
        .execute(&pool)
        .await?;

    Ok(())
}
