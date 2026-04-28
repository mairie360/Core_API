use crate::database::sessions::get_active_sessions::GetActiveSessionsQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_active_sessions_query(
    view: GetActiveSessionsQueryView,
    pool: PgPool,
) -> Result<Vec<Session>, DatabaseError> {
    let result: Vec<Session> = sqlx::query_as::<_, Session>(&view.get_request())
        .bind(view.get_user_id() as i64)
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
