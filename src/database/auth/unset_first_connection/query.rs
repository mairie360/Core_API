use crate::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use mairie360_api_lib::database::{db_interface::DatabaseQueryView, errors::DatabaseError};
use sqlx::PgPool;

pub async fn unset_first_connection_query(
    view: UnsetFirstConnectionQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.password())
        .bind(view.user_id() as i32)
        .execute(&pool)
        .await?;

    Ok(())
}
