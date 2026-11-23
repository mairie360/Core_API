//! Services of the mairie (MAIR-923, Database `mair-923-01`): a name, unique whatever its case, and
//! the agents attached to it through `users.service_id`.
pub mod view;

pub use view::{
    CreateServiceQueryView, DeleteServiceQueryView, ListServicesQueryView, Service,
    SetUserServiceQueryView, UpdateServiceQueryView,
};
