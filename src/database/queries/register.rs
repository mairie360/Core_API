use crate::database::query_views::RegisterUserQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn register_query(
    view: RegisterUserQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.get_first_name())
        .bind(view.get_last_name())
        .bind(view.get_email())
        .bind(view.get_password())
        .bind(view.get_phone_number())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
