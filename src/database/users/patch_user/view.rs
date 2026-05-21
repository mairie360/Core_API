use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct PatchUserQueryView {
    id: u64,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone_number: Option<String>,
}

impl PatchUserQueryView {
    pub fn new(
        id: u64,
        first_name: Option<&str>,
        last_name: Option<&str>,
        email: Option<&str>,
        phone_number: Option<&str>,
    ) -> Self {
        Self {
            id,
            first_name: first_name.map(|s| s.to_string()),
            last_name: last_name.map(|s| s.to_string()),
            email: email.map(|s| s.to_string()),
            phone_number: phone_number.map(|s| s.to_string()),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }
    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
}

impl DatabaseQueryView for PatchUserQueryView {
    fn get_request(&self) -> String {
        let mut request = "UPDATE users SET ".to_string();
        if let Some(first_name) = &self.first_name {
            request.push_str(&format!("first_name = '{}', ", first_name));
        }
        if let Some(last_name) = &self.last_name {
            request.push_str(&format!("last_name = '{}', ", last_name));
        }
        if let Some(email) = &self.email {
            request.push_str(&format!("email = '{}', ", email));
        }
        if let Some(phone_number) = &self.phone_number {
            request.push_str(&format!("phone_number = '{}', ", phone_number));
        }
        request.push_str(&format!("WHERE id = {}", self.id));
        request.push_str(" RETURNING true");
        request
    }
}

impl Display for PatchUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.phone_number {
            Some(_) => write!(f, "PatchUserQueryView: id = {:?}, first_name = {:?}, last_name = {:?}, email = {:?}, phone_number = {:?}", self.id(), self.first_name(), self.last_name(), self.email(), self.phone_number()),
            None => write!(f, "PatchUserQueryView: id = {:?}, first_name = {:?}, last_name = {:?}, email = {:?}", self.id(), self.first_name(), self.last_name(), self.email()),
        }
    }
}
