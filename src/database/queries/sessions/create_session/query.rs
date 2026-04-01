use crate::database::queries::sessions::create_session::CreateSessionQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn create_session_query(
    view: CreateSessionQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.get_user_id() as i64)
        .bind(view.get_token_hash())
        .bind(view.get_device_info())
        .bind(view.get_ip_address())
        .execute(&pool)
        .await?;

    Ok(())
}
