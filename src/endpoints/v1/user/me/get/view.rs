use crate::database::{
    groups::get_group::Group, users::get_user_by_id::GetUserByIdQueryResultView,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Profile of the signed-in user.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetMeResponseView {
    /// First name of the signed-in user.
    #[schema(example = "Jean")]
    first_name: String,
    /// Last name of the signed-in user.
    #[schema(example = "Dupont")]
    last_name: String,
    /// E-mail address, unique on the platform.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Phone number in E.164 (`+33612345678`), or `null` when the user has none. A legacy number
    /// that could not be attributed to a country (MAIR-480) is returned as stored, digits only,
    /// with `phone_country` `null`.
    #[schema(example = "+33612345678")]
    phone: Option<String>,
    /// ISO 3166-1 alpha-2 code of the phone number's country (`FR`), `null` when `phone` is.
    #[schema(example = "FR")]
    phone_country: Option<String>,
    /// New e-mail address asked through `PATCH /api/v1/user/me/` and still waiting for its
    /// confirmation link to be used (valid one hour), `null` when there is none. `email` stays the
    /// address of the account until then.
    #[schema(format = Email, example = "jean.dupont@nouvelle-adresse.fr")]
    pending_email: Option<String>,
    /// Status of the account as stored in the database.
    #[schema(example = "active")]
    status: String,
    /// Name of the user's role: a user holds one role (MAIR-564), the one carried by the `role`
    /// claim of their JWT. Empty string when the user has none (never `null`).
    #[schema(example = "agent")]
    role: String,
    /// Deprecated: `role` as a list, kept for the clients written when a user could hold several
    /// roles. At most one entry, empty when the user has none. Read `role` instead.
    #[schema(example = json!(["agent"]))]
    #[deprecated(note = "read `role`, a user holds one role")]
    roles: Vec<String>,
    /// Groups the user is a member of. Empty if they belong to none.
    groups: Vec<Group>,
}

impl GetMeResponseView {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    #[allow(deprecated)]
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<String>,
        phone_country: Option<&str>,
        status: &str,
        roles: Vec<String>,
        groups: Vec<Group>,
        pending_email: Option<String>,
    ) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone,
            phone_country: phone_country.map(std::string::ToString::to_string),
            status: status.to_string(),
            pending_email,
            role: roles.first().cloned().unwrap_or_default(),
            roles,
            groups,
        }
    }

    #[must_use]
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    #[must_use]
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[must_use]
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }

    #[must_use]
    pub fn phone_country(&self) -> Option<&str> {
        self.phone_country.as_deref()
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    #[must_use]
    #[allow(deprecated)]
    pub fn roles(&self) -> &[String] {
        &self.roles
    }

    #[must_use]
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
}

impl Display for GetMeResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetMeResponseView {{ first_name: {}, last_name: {}, email: {}, phone: {:?}, status: {}, role: {}, groups: {} }}",
            self.first_name,
            self.last_name,
            self.email,
            self.phone,
            self.status,
            self.role,
            self.groups.len(),
        )
    }
}

impl From<GetUserByIdQueryResultView> for GetMeResponseView {
    #[allow(deprecated)]
    fn from(query_result: GetUserByIdQueryResultView) -> Self {
        Self {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result.phone_e164(),
            phone_country: query_result
                .phone_country()
                .map(std::string::ToString::to_string),
            status: query_result.status().to_string(),
            pending_email: None,
            role: String::new(),
            roles: Vec::new(),
            groups: Vec::new(),
        }
    }
}
