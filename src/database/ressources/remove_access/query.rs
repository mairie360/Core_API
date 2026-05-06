use crate::database::ressources::remove_access::view::RemoveAccessQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn remove_access_query(
    view: RemoveAccessQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.id() as i64)
        .fetch_one(&pool)
        .await?;

    Ok(())
}
