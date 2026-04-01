use crate::database::queries::sessions::get_active_session::GetActiveSessionQueryView;
use crate::database::queries::sessions::Session;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_active_session_query(
    view: GetActiveSessionQueryView,
    pool: PgPool,
) -> Result<Option<Session>, DatabaseError> {
    let result: Option<Session> = sqlx::query_as::<_, Session>(&view.get_request())
        .bind(view.get_user_id() as i64)
        .bind(view.get_ip())
        .bind(view.get_device_info())
        .fetch_optional(&pool)
        .await?;

    Ok(result)
}
