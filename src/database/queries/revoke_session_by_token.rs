use crate::database::query_views::RevokeSessionByTokenQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn revoke_session_by_token_query(
    view: RevokeSessionByTokenQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.get_revoked_at())
        .bind(view.get_user_id() as i64)
        .bind(view.get_token_hash())
        .execute(&pool)
        .await?;

    Ok(())
}
