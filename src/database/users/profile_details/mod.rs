//! Profile details of the signed-in user shown on the Settings profile: biography, job title,
//! photo and the service the administration attached them to.
mod view;

pub use view::{
    ClearPhotoQueryView, GetPhotoQueryView, GetProfileDetailsQueryView,
    PatchProfileDetailsQueryView, PhotoRow, ProfileDetails, SetPhotoQueryView,
};
