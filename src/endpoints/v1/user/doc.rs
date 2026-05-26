use crate::endpoints::v1::user::{id::doc::IdDoc, me::doc::MeDoc};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(nest(
    (path = "/me", api = MeDoc, tags = ["Users"]),
    (path = "/{id}", api = IdDoc, tags = ["Users"]),
))]
pub struct UserDoc;
