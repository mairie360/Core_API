use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

use crate::database::auth::change_password::ChangePasswordQueryView;

pub async fn change_password_query(
    view: ChangePasswordQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.get_password())
        .bind(view.get_user_id() as i32)
        .execute(&pool)
        .await?;

    Ok(())
}
