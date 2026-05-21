pub mod database;
pub mod endpoints;

use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use std::env;

// Structure pour stocker les informations de destination
pub struct EmailDestination {
    pub from: String,
    pub to: String,
}

// Type alias pour clarifier le type du mailer de la lib 'lettre'
pub type SmtpMailer = AsyncSmtpTransport<Tokio1Executor>;

pub fn get_email_sender() -> Result<String, Box<dyn std::error::Error>> {
    // Récupère les variables d'environnement, avec des valeurs de secours (fallback) au cas où
    Ok(env::var("EMAIL_FROM").unwrap_or_else(|_| "Beta App <noreply@votrebeta.com>".to_string()))
}

pub fn build_email(
    destination: &EmailDestination,
    subject: &str,
    body_content: &str,
) -> Result<Message, Box<dyn std::error::Error>> {
    let email = Message::builder()
        .from(destination.from.parse()?)
        .to(destination.to.parse()?)
        .subject(subject)
        .body(body_content.to_string())?;

    Ok(email)
}

pub async fn send_email(email: Message) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Récupération des identifiants SMTP
    let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
    let smtp_port = env::var("SMTP_PORT").unwrap_or_else(|_| "1025".to_string());
    let username = env::var("SMTP_USERNAME").unwrap_or_default();
    let password = env::var("SMTP_PASSWORD").unwrap_or_default();

    let port: u16 = smtp_port.parse().unwrap_or(1025);

    // 2. Configuration dynamique du transporteur
    let mailer: SmtpMailer =
        if smtp_host == "mailpit" || smtp_host == "localhost" || username.is_empty() {
            // En développement local (Mailpit), on se connecte sans chiffrement TLS
            // CORRECTION ICI : builder(...) au lieu de builder_some(...)
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_host)
                .port(port)
                .build()
        } else {
            // En production (Resend, SendGrid...), on utilise STARTTLS avec authentification
            let creds = Credentials::new(username, password);
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_host)?
                .credentials(creds)
                .port(port)
                .build()
        };

    // 3. Envoi effectif
    mailer.send(email).await?;

    Ok(())
}
