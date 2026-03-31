use crate::database::query_views::RevokeSessionByIdQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn revoke_session_by_id_query(
    view: RevokeSessionByIdQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.get_revoked_at())
        .bind(view.get_user_id() as i64)
        .bind(view.get_id())
        .execute(&pool)
        .await?;

    Ok(())
}
