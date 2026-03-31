use crate::database::query_views::RevokePreviousSessionQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn revoke_previous_session_query(
    view: RevokePreviousSessionQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.get_user_id() as i64)
        .bind(view.get_ip())
        .bind(view.get_device_info())
        .execute(&pool)
        .await?;

    Ok(())
}
