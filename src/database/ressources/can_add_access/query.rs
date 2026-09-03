use crate::database::ressources::can_add_access::CanAddAccessQueryView;
use crate::database::ressources::is_owner::{is_owner_query, IsOwnerQueryView};
use crate::endpoints::v1::ressources::AccessType;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn can_add_access_query(
    view: CanAddAccessQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    if view.access_type() == AccessType::Error {
        return Ok(false);
    }
    if is_owner_query(
        IsOwnerQueryView::new(view.owner_id(), view.ressource_id(), view.ressource_type()),
        smart_db,
    )
    .await?
    {
        return Ok(true);
    }
    Ok(false)
}
