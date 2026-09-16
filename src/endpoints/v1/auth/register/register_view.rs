use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Création de compte par l'utilisateur lui-même.
#[derive(Deserialize, ToSchema)]
pub struct RegisterView {
    /// Prénom de l'utilisateur.
    #[schema(example = "Jean")]
    first_name: String,
    /// Nom de famille de l'utilisateur.
    #[schema(example = "Dupont")]
    last_name: String,
    /// Adresse e-mail, unique sur la plateforme. Doit contenir un `@` et un domaine pointé.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Mot de passe initial, d'au moins 8 caractères.
    #[schema(format = Password, min_length = 8, example = "MotDePasse!123")]
    password: String,
    /// Numéro de téléphone facultatif. S'il est fourni : au moins 10 caractères, chiffres
    /// uniquement (ni espace, ni `+`, ni séparateur).
    #[schema(min_length = 10, pattern = r"^\d{10,}$", example = "0612345678")]
    phone_number: Option<String>,
}

impl RegisterView {
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
}

impl Display for RegisterView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegisterView {{ first_name: {}, last_name: {}, email: {}, password: {}, phone_number: {:?} }}",
            self.first_name, self.last_name, self.email, self.password, self.phone_number
        )
    }
}
