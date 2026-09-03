use crate::database::ressources::get_access_by_ressource::{Access, GetAccessByRessourceQueryView};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_access_by_ressource(
    view: GetAccessByRessourceQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<Access>, ApiLibError> {
    let result: Vec<Access> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
