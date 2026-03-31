use crate::database::queries_result_views::Session;
use crate::database::query_views::GetActiveSessionQueryView;
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
