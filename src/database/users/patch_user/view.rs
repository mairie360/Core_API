use crate::database::ids::id_to_sql;
use crate::phone::PhoneChange;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Partial update of an account: only the fields given as `Some` are written.
///
/// The SQL text is one static literal (MAIR-390): each column is paired with a boolean telling
/// whether it changes, `CASE WHEN <flag> THEN <value> ELSE <column> END`. It used to be rebuilt
/// for every call and leaked to get a `&'static str`.
#[derive(Debug, serde::Deserialize)]
pub struct PatchUserQueryView {
    id: u64,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    #[serde(skip, default = "keep_phone")]
    phone: PhoneChange,
    password: Option<String>,
    params: Vec<QueryParam>,
}

const fn keep_phone() -> PhoneChange {
    PhoneChange::Keep
}

/// Flag and value parameters of an optional column (empty text when unchanged, never written).
fn column_params(value: Option<&str>) -> [QueryParam; 2] {
    [
        QueryParam::Bool(value.is_some()),
        QueryParam::Text(value.unwrap_or_default().to_string()),
    ]
}

impl PatchUserQueryView {
    #[must_use]
    pub fn new(
        id: u64,
        first_name: Option<&str>,
        last_name: Option<&str>,
        email: Option<&str>,
        phone: &PhoneChange,
        password: Option<&str>,
    ) -> Self {
        // The phone is one flag and two values (country, national number); an empty value is
        // written NULL, which is how `Clear` removes both.
        let (country, national) = match phone {
            PhoneChange::Set(phone) => (phone.country(), phone.national()),
            PhoneChange::Keep | PhoneChange::Clear => ("", ""),
        };
        let params = [first_name, last_name, email]
            .into_iter()
            .flat_map(column_params)
            .chain([
                QueryParam::Bool(*phone != PhoneChange::Keep),
                QueryParam::Text(country.to_string()),
                QueryParam::Text(national.to_string()),
            ])
            .chain(column_params(password))
            .chain(std::iter::once(QueryParam::I32(id_to_sql(id))))
            .collect();

        Self {
            id,
            first_name: first_name.map(std::string::ToString::to_string),
            last_name: last_name.map(std::string::ToString::to_string),
            email: email.map(std::string::ToString::to_string),
            phone: phone.clone(),
            password: password.map(std::string::ToString::to_string),
            params,
        }
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
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
    #[must_use]
    pub const fn phone(&self) -> &PhoneChange {
        &self.phone
    }
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// True when no field was given: there is nothing to write.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        self.first_name.is_none()
            && self.last_name.is_none()
            && self.email.is_none()
            && matches!(self.phone, PhoneChange::Keep)
            && self.password.is_none()
    }
}

impl ApiRequestDto for PatchUserQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET \
            first_name = CASE WHEN $1 THEN $2 ELSE first_name END, \
            last_name = CASE WHEN $3 THEN $4 ELSE last_name END, \
            email = CASE WHEN $5 THEN $6 ELSE email END, \
            phone_country = CASE WHEN $7 THEN NULLIF($8, '') ELSE phone_country END, \
            phone_number = CASE WHEN $7 THEN NULLIF($9, '') ELSE phone_number END, \
            password = CASE WHEN $10 THEN $11 ELSE password END \
         WHERE id = $12"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchUserQueryView {
    // Only say which fields change: the e-mail, phone number and password never reach the logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const fn changed(set: bool) -> &'static str {
            if set {
                "[PROTECTED]"
            } else {
                "unchanged"
            }
        }
        write!(
            f,
            "PatchUserQueryView: id = {:?}, first_name = {:?}, last_name = {:?}, email = {}, phone = {}, password = {}",
            self.id(),
            self.first_name(),
            self.last_name(),
            changed(self.email().is_some()),
            changed(!matches!(self.phone, PhoneChange::Keep)),
            changed(self.password().is_some()),
        )
    }
}
