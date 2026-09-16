use crate::database::groups::get_group::Group;
use utoipa::ToSchema;

/// Détail d'un groupe.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupResultView {
    /// Le groupe demandé.
    group: Group,
}

impl GetGroupResultView {
    pub const fn new(group: Group) -> Self {
        Self { group }
    }
}
