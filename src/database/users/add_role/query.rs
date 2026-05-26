use crate::database::users::add_role::AddRolesQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn add_role_query(view: AddRolesQueryView, pool: PgPool) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.user_id() as i32)
        .bind(view.role_id() as i32)
        .execute(&pool)
        .await?;

    Ok(())
}
