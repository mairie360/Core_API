use crate::database::sessions::get_sessions_by_user::GetSessionsByUserQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_sessions_by_user_query(
    view: GetSessionsByUserQueryView,
    pool: PgPool,
) -> Result<Vec<Session>, DatabaseError> {
    let result: Vec<Session> = sqlx::query_as::<_, Session>(&view.get_request())
        .bind(view.get_user_id() as i64)
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
