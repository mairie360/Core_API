use crate::database::ressources::can_add_access::CanAddAccessQueryView;
use crate::database::ressources::is_owner::{is_owner_query, IsOwnerQueryView};
use crate::endpoints::v1::ressources::AccessType;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn can_add_access_query(
    view: CanAddAccessQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    if view.access_type() == AccessType::Error {
        return Ok(false);
    }
    if is_owner_query(
        IsOwnerQueryView::new(view.owner_id(), view.ressource_id(), view.ressource_type()),
        pool,
    )
    .await?
    {
        return Ok(true);
    }
    eprintln!("TODO");
    Ok(false)
}
