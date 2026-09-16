use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct PatchUserView {
    /// Nouveau prénom. Absent ou `null` pour ne pas y toucher.
    #[schema(example = "Jean")]
    first_name: Option<String>,
    /// Nouveau nom de famille. Absent ou `null` pour ne pas y toucher.
    #[schema(example = "Dupont")]
    last_name: Option<String>,
    /// Nouvelle adresse e-mail. Elle doit rester unique, sinon l'appel échoue en `404`.
    #[schema(format = Email, example = "j.dupont@mairie360.fr")]
    email: Option<String>,
    /// Nouveau numéro de téléphone. Absent ou `null` pour ne pas y toucher.
    #[schema(example = "0798765432")]
    phone_number: Option<String>,
    /// Nouveau mot de passe, écrit **sans validation de longueur**. Préférer
    /// `PATCH /api/v1/admin/users/{userId}/password`, qui impose 8 à 255 caractères.
    #[schema(format = Password, example = "NouveauMotDePasse!123")]
    password: Option<String>,
}

impl PatchUserView {
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

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl Display for PatchUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchUserView {{ first_name: {}, last_name: {}, email: {}, phone_number: {:?}, password: {:?} }}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or(""),
            self.email.as_deref().unwrap_or(""),
            self.phone_number.as_deref().unwrap_or(""),
            self.password.as_deref().unwrap_or("")
        )
    }
}
