use crate::endpoints::v1::user::me::nullable::double_option;
use crate::endpoints::validation::{
    check_description, check_email, check_label, check_opaque, check_optional, check_phone_change,
    Validate, ValidationError, MAX_BIOGRAPHY_LENGTH, MAX_JOB_TITLE_LENGTH, MAX_NAME_LENGTH,
    MAX_PASSWORD_LENGTH,
};
use crate::phone::PhoneChange;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Partial update of the connected user's profile: only the fields sent are written.
#[allow(clippy::option_option)]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchMeView {
    /// New first name. Absent or `null` to leave it unchanged.
    #[schema(min_length = 1, max_length = 64, example = "Jean")]
    first_name: Option<String>,
    /// New last name. Absent or `null` to leave it unchanged.
    #[schema(min_length = 1, max_length = 64, example = "Dupont")]
    last_name: Option<String>,
    /// New e-mail address. Absent or `null` to leave it unchanged.
    #[schema(max_length = 320, format = Email, example = "jean.dupont@mairie360.fr")]
    email: Option<String>,
    /// New phone number, as typed: national format (`06 12 34 56 78`, digits, spaces, `.`, `-`,
    /// `(`, `)`) with `phone_country`, or E.164 (`+33612345678`, `phone_country` optional). It
    /// must be a valid number of that country. Absent keeps the stored phone; `null` or `""`
    /// removes it.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 32, example = "06 12 34 56 78")]
    phone: Option<Option<String>>,
    /// ISO 3166-1 alpha-2 code of the country `phone` is typed for (`FR`, `BE`...). Required with
    /// a national number, refused without `phone`. The stored country is the number's own: a
    /// `+262` number sent with `FR` is stored `RE`.
    #[schema(min_length = 2, max_length = 2, pattern = "^[A-Z]{2}$", example = "FR")]
    #[serde(default)]
    phone_country: Option<String>,
    /// Current password of the account. Required when `email` is sent (MAIR-390): a stolen
    /// session must not be enough to move the account to another mailbox. Ignored otherwise.
    #[schema(max_length = 255, format = Password, example = "MotDePasse!123")]
    #[serde(default)]
    current_password: Option<String>,
    /// New biography, plain text: line breaks allowed, at most 1000 characters. Absent keeps it;
    /// `null` or a blank text removes it. Stored trimmed.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 1000, example = "Roads department agent since 2018.")]
    biography: Option<Option<String>>,
    /// New function (job title), plain text on one line, at most 100 characters. Absent keeps it;
    /// `null` or a blank text removes it. Stored trimmed.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 100, example = "Head of the roads department")]
    job_title: Option<Option<String>>,
}

/// `null` and blank texts clear an optional text; any other value is stored trimmed.
#[allow(clippy::option_option)]
fn clearable(value: Option<&Option<String>>) -> Option<Option<&str>> {
    value.map(|text| {
        text.as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
    })
}

impl PatchMeView {
    #[must_use]
    pub const fn new(
        first_name: Option<String>,
        last_name: Option<String>,
        email: Option<String>,
        phone: Option<Option<String>>,
        phone_country: Option<String>,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email,
            phone,
            phone_country,
            current_password: None,
            biography: None,
            job_title: None,
        }
    }

    /// Sets the biography and the job title of the update (`Some(None)` removes one).
    #[must_use]
    #[allow(clippy::option_option)]
    pub fn with_details(
        mut self,
        biography: Option<Option<String>>,
        job_title: Option<Option<String>>,
    ) -> Self {
        self.biography = biography;
        self.job_title = job_title;
        self
    }

    /// What the request does to the biography: `None` keeps it, `Some(None)` removes it.
    #[must_use]
    #[allow(clippy::option_option)]
    pub fn biography(&self) -> Option<Option<&str>> {
        clearable(self.biography.as_ref())
    }

    /// What the request does to the job title: `None` keeps it, `Some(None)` removes it.
    #[must_use]
    #[allow(clippy::option_option)]
    pub fn job_title(&self) -> Option<Option<&str>> {
        clearable(self.job_title.as_ref())
    }

    #[must_use]
    pub fn current_password(&self) -> Option<&str> {
        self.current_password.as_deref()
    }

    #[must_use]
    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }

    #[must_use]
    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }

    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// What the request does to the phone.
    ///
    /// # Errors
    ///
    /// Returns the `400` of an invalid phone or country (already refused by [`Validate`]).
    pub fn phone_change(&self) -> Result<PhoneChange, ValidationError> {
        check_phone_change(
            "phone",
            self.phone.as_ref().map(Option::as_deref),
            self.phone_country.as_deref(),
        )
    }
}

impl Display for PatchMeView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchMeView {{ first_name: {:?}, last_name: {:?}, email: [PROTECTED], phone: {} }}",
            self.first_name,
            self.last_name,
            if self.phone.is_some() {
                "[PROTECTED]"
            } else {
                "unchanged"
            }
        )
    }
}

impl Validate for PatchMeView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.first_name.as_deref(), |name| {
            check_label("first_name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.last_name.as_deref(), |name| {
            check_label("last_name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.email.as_deref(), |email| check_email("email", email))?;
        if let Some(Some(biography)) = self.biography() {
            check_description("biography", biography, MAX_BIOGRAPHY_LENGTH)?;
        }
        if let Some(Some(job_title)) = self.job_title() {
            check_label("job_title", job_title, MAX_JOB_TITLE_LENGTH)?;
        }
        self.phone_change()?;
        if self.email.is_some() && self.current_password.is_none() {
            return Err(ValidationError::new(
                "current_password",
                "is required to change the e-mail address",
            ));
        }
        check_optional(self.current_password.as_deref(), |password| {
            check_opaque("current_password", password, MAX_PASSWORD_LENGTH)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> PatchMeView {
        serde_json::from_str(body).unwrap()
    }

    #[test]
    fn mair_1231_absent_keeps_null_and_blank_clear_a_value_is_trimmed() {
        let view = parse("{}");
        assert_eq!(view.biography(), None);
        assert_eq!(view.job_title(), None);
        let view = parse(r#"{ "biography": null, "job_title": "  \t " }"#);
        assert_eq!(view.biography(), Some(None));
        assert_eq!(view.job_title(), Some(None));
        let view = parse(r#"{ "biography": " Bio\nline two ", "job_title": " Chef " }"#);
        assert_eq!(view.biography(), Some(Some("Bio\nline two")));
        assert_eq!(view.job_title(), Some(Some("Chef")));
        assert!(view.validate().is_ok());
    }

    #[test]
    fn mair_1231_job_title_is_one_line_of_at_most_100_characters() {
        assert!(
            parse(&format!(r#"{{ "job_title": "{}" }}"#, "a".repeat(100)))
                .validate()
                .is_ok()
        );
        let error = parse(&format!(r#"{{ "job_title": "{}" }}"#, "a".repeat(101)))
            .validate()
            .unwrap_err();
        assert!(error.to_string().contains("job_title"));
        assert!(parse(r#"{ "job_title": "Chef\nde service" }"#)
            .validate()
            .is_err());
    }

    #[test]
    fn mair_1231_biography_allows_line_breaks_up_to_1000_characters() {
        assert!(
            parse(&format!(r#"{{ "biography": "{}" }}"#, "a".repeat(1000)))
                .validate()
                .is_ok()
        );
        let error = parse(&format!(r#"{{ "biography": "{}" }}"#, "a".repeat(1001)))
            .validate()
            .unwrap_err();
        assert!(error.to_string().contains("biography"));
        assert!(parse(r#"{ "biography": "Bio\u0007" }"#).validate().is_err());
    }
}
