use crate::database::sessions::get_sessions::GetSessionsQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_sessions_query(
    view: GetSessionsQueryView,
    pool: PgPool,
) -> Result<Vec<Session>, DatabaseError> {
    let result = sqlx::query_as::<_, Session>(&view.get_request())
        .bind(view.id())
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
