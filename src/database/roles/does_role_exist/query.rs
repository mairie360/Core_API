use crate::database::roles::does_role_exist::view::DoesRoleExistQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn does_role_exist_query(
    view: DoesRoleExistQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result: bool = sqlx::query_scalar::<_, bool>(&view.get_request())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
