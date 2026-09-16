use crate::endpoints::v1::user::{get::doc::DirectoryDoc, id::doc::IdDoc, me::doc::MeDoc};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/", api = DirectoryDoc),
    (path = "/me", api = MeDoc),
    (path = "/{id}", api = IdDoc),
))]
pub struct UserDoc;
