//! Photo of the signed-in user, shown on their Settings profile. The front crops and compresses it
//! (256 × 256 WebP); the API only accepts a WebP image of at most [`MAX_PHOTO_BYTES`] and stores it
//! in `users.photo`. It has its own route so that `GET /api/v1/user/me/`, read on every page load,
//! does not carry it.

use crate::database::users::profile_details::{
    ClearPhotoQueryView, GetPhotoQueryView, PhotoRow, SetPhotoQueryView,
};
use actix_web::http::StatusCode;
use actix_web::{delete, get, put, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

/// Largest photo accepted, in bytes (100 KiB).
pub const MAX_PHOTO_BYTES: usize = 100 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoError {
    /// The body is empty or not a WebP image.
    NotWebp,
    /// The body is larger than [`MAX_PHOTO_BYTES`].
    TooLarge,
    /// The user has no photo.
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for PhotoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWebp => write!(f, "Invalid `photo`: must be a WebP image"),
            Self::TooLarge => write!(f, "Invalid `photo`: must be at most 102400 bytes"),
            Self::NotFound => write!(f, "The user has no photo."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for PhotoError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotWebp => StatusCode::BAD_REQUEST,
            Self::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Checks that `photo` is a WebP image the profile can store: a RIFF container of type `WEBP`
/// (its first 12 bytes, whatever the `Content-Type` header says), at most [`MAX_PHOTO_BYTES`].
///
/// # Errors
///
/// [`PhotoError::TooLarge`] over the size, [`PhotoError::NotWebp`] for anything else than WebP.
pub fn check_photo(photo: &[u8]) -> Result<(), PhotoError> {
    if photo.len() > MAX_PHOTO_BYTES {
        return Err(PhotoError::TooLarge);
    }
    let is_webp = photo.len() > 12 && &photo[0..4] == b"RIFF" && &photo[8..12] == b"WEBP";
    if is_webp {
        Ok(())
    } else {
        Err(PhotoError::NotWebp)
    }
}

fn db_error(e: impl std::fmt::Display) -> PhotoError {
    tracing::error!("Photo DB Error: {e}");
    PhotoError::DatabaseError
}

#[utoipa::path(
    put,
    path = "/photo",
    summary = "Set one's own photo",
    description = "Stores the photo of the user of the JWT, replacing the previous one. The body \
                   is the raw image: a WebP of at most 102400 bytes (the fronts crop it to 256 × 256 \
                   first). The format is read from the image itself, not from `Content-Type`. \
                   `GET /api/v1/user/me/` then answers `has_photo: true`.",
    request_body(content = Vec<u8>, content_type = "image/webp", description = "WebP image, 102400 bytes at most."),
    responses(
        (status = 204, description = "Photo stored. Empty body."),
        (
            status = 400,
            description = "The body is empty or not a WebP image.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `photo`: must be a WebP image")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 413,
            description = "The image is larger than 102400 bytes.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `photo`: must be at most 102400 bytes")
        ),
        (
            status = 500,
            description = "Database error while storing the photo.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[put("/photo")]
pub async fn set_my_photo(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    body: web::Bytes,
) -> Result<impl Responder, PhotoError> {
    check_photo(&body)?;
    state
        .get_smart_db()
        .execute(SetPhotoQueryView::new(auth_user.id, &body))
        .await
        .map_err(db_error)?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    get,
    path = "/photo",
    summary = "Read one's own photo",
    description = "Returns the photo of the user of the JWT as stored by `PUT /api/v1/user/me/photo` \
                   (`image/webp`). Not cached by shared caches.",
    responses(
        (status = 200, description = "The photo.", body = Vec<u8>, content_type = "image/webp"),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "The user has no photo.",
            body = String,
            content_type = "text/plain",
            example = json!("The user has no photo.")
        ),
        (
            status = 500,
            description = "Database error while reading the photo.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[get("/photo")]
pub async fn get_my_photo(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PhotoError> {
    let row: PhotoRow = state
        .get_smart_db()
        .fetch_one(&GetPhotoQueryView::new(auth_user.id))
        .await
        .map_err(db_error)?;
    let photo = row.bytes().ok_or(PhotoError::NotFound)?;
    Ok(HttpResponse::Ok()
        .content_type("image/webp")
        .insert_header(("Cache-Control", "private, no-cache"))
        .body(photo))
}

#[utoipa::path(
    delete,
    path = "/photo",
    summary = "Remove one's own photo",
    description = "Removes the photo of the user of the JWT. Answers `204` whether there was one or not.",
    responses(
        (status = 204, description = "No photo left. Empty body."),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error while removing the photo.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[delete("/photo")]
pub async fn delete_my_photo(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, PhotoError> {
    state
        .get_smart_db()
        .execute(ClearPhotoQueryView::new(auth_user.id))
        .await
        .map_err(db_error)?;
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn webp(len: usize) -> Vec<u8> {
        let mut image = b"RIFF\0\0\0\0WEBPVP8 ".to_vec();
        image.resize(len, 0);
        image
    }

    #[test]
    fn mair_1231_accepts_a_webp_up_to_100_kib() {
        assert_eq!(check_photo(&webp(64)), Ok(()));
        assert_eq!(check_photo(&webp(MAX_PHOTO_BYTES)), Ok(()));
    }

    #[test]
    fn mair_1231_refuses_a_photo_over_100_kib() {
        assert_eq!(
            check_photo(&webp(MAX_PHOTO_BYTES + 1)),
            Err(PhotoError::TooLarge)
        );
    }

    #[test]
    fn mair_1231_refuses_what_is_not_webp() {
        assert_eq!(check_photo(b""), Err(PhotoError::NotWebp));
        assert_eq!(check_photo(b"RIFF\0\0\0\0WEBP"), Err(PhotoError::NotWebp));
        assert_eq!(
            check_photo(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
            Err(PhotoError::NotWebp)
        );
        assert_eq!(
            check_photo(b"RIFF\0\0\0\0WAVEfmt "),
            Err(PhotoError::NotWebp)
        );
        assert_eq!(
            check_photo(b"<svg xmlns='http://www.w3.org/2000/svg'/>"),
            Err(PhotoError::NotWebp)
        );
    }
}
