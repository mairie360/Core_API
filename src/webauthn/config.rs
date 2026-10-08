use mairie360_api_lib::env_manager::get_env_var;
use url::Url;
use webauthn_rs::prelude::{Webauthn, WebauthnBuilder, WebauthnError};

/// Name shown by the browser when `WEBAUTHN_RP_NAME` is not set.
const DEFAULT_RP_NAME: &str = "Mairie 360";

/// Relying party settings of the passkey ceremonies.
///
/// Passkeys are optional: when the relying party id or its origin is missing,
/// [`WebauthnConfig::from_env`] returns `None` and the passkey routes answer `503`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebauthnConfig {
    rp_id: String,
    rp_name: String,
    origins: Vec<Url>,
}

impl WebauthnConfig {
    /// Builds a configuration from its raw parts.
    ///
    /// `rp_id` is the domain the passkeys are bound to (the instance's domain, e.g.
    /// `prod.mairie360-eip.fr`): a passkey registered for it works from any origin of that
    /// domain listed in `origins`, and from nowhere else. `origins` are the browser origins the
    /// sign-in page is served from (`https://login.prod.mairie360-eip.fr`); the first one is the
    /// primary one, every one of them must be the relying party id or a subdomain of it.
    #[must_use]
    pub fn new(rp_id: &str, rp_name: Option<&str>, origins: Vec<Url>) -> Self {
        Self {
            rp_id: rp_id.trim().to_string(),
            rp_name: rp_name
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or(DEFAULT_RP_NAME)
                .to_string(),
            origins,
        }
    }

    /// Reads `WEBAUTHN_RP_ID`, `WEBAUTHN_RP_ORIGIN` (one or more origins separated by commas)
    /// and `WEBAUTHN_RP_NAME` (optional).
    ///
    /// Returns `None`, which disables the passkeys, when the id or the origins are missing or
    /// empty; an origin that is not a valid URL is dropped with a warning.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let rp_id = get_env_var("WEBAUTHN_RP_ID").filter(|value| !value.trim().is_empty());
        let raw_origins =
            get_env_var("WEBAUTHN_RP_ORIGIN").filter(|value| !value.trim().is_empty());
        match (rp_id, raw_origins) {
            (Some(rp_id), Some(raw_origins)) => {
                let origins: Vec<Url> = raw_origins
                    .split(',')
                    .map(str::trim)
                    .filter(|origin| !origin.is_empty())
                    .filter_map(|origin| {
                        Url::parse(origin)
                            .inspect_err(|_| {
                                tracing::warn!(
                                    "WEBAUTHN_RP_ORIGIN holds a value that is not a URL: ignored."
                                );
                            })
                            .ok()
                    })
                    .collect();
                if origins.is_empty() {
                    tracing::warn!("Passkeys disabled: WEBAUTHN_RP_ORIGIN holds no valid URL.");
                    return None;
                }
                Some(Self::new(
                    &rp_id,
                    get_env_var("WEBAUTHN_RP_NAME").as_deref(),
                    origins,
                ))
            }
            (None, None) => None,
            _ => {
                tracing::warn!(
                    "Passkeys disabled: WEBAUTHN_RP_ID and WEBAUTHN_RP_ORIGIN must both be set."
                );
                None
            }
        }
    }

    #[must_use]
    pub fn rp_id(&self) -> &str {
        &self.rp_id
    }

    #[must_use]
    pub fn rp_name(&self) -> &str {
        &self.rp_name
    }

    #[must_use]
    pub fn origins(&self) -> &[Url] {
        &self.origins
    }

    /// The relying party to register as app data.
    ///
    /// # Errors
    ///
    /// [`WebauthnError::Configuration`] when an origin is not the relying party id or one of its
    /// subdomains: a deployment error, the API must not start with it.
    pub fn build(&self) -> Result<Webauthn, WebauthnError> {
        let (primary, others) = self
            .origins
            .split_first()
            .ok_or(WebauthnError::Configuration)?;
        let mut builder = WebauthnBuilder::new(&self.rp_id, primary)?.rp_name(&self.rp_name);
        for origin in others {
            if !is_origin_of(origin, &self.rp_id) {
                tracing::error!("WEBAUTHN_RP_ORIGIN holds an origin outside WEBAUTHN_RP_ID.");
                return Err(WebauthnError::Configuration);
            }
            builder = builder.append_allowed_origin(origin);
        }
        builder.build()
    }
}

/// Whether `origin` is served by `rp_id` or one of its subdomains (the rule of
/// [`WebauthnBuilder::new`], which only checks the primary origin).
fn is_origin_of(origin: &Url, rp_id: &str) -> bool {
    origin
        .domain()
        .is_some_and(|domain| domain == rp_id || domain.ends_with(&format!(".{rp_id}")))
}

#[cfg(test)]
mod tests {
    use super::WebauthnConfig;
    use url::Url;

    fn url(value: &str) -> Url {
        Url::parse(value).unwrap()
    }

    #[test]
    fn builds_a_relying_party_for_the_login_front_of_the_domain() {
        let config = WebauthnConfig::new(
            "mairie360.test",
            None,
            vec![
                url("https://login.mairie360.test"),
                url("https://mairie360.test"),
            ],
        );
        assert_eq!(config.rp_name(), "Mairie 360");
        let webauthn = config.build().expect("valid configuration");
        assert_eq!(webauthn.get_allowed_origins().len(), 2);
    }

    #[test]
    fn refuses_an_origin_outside_the_relying_party() {
        for origins in [
            vec![url("https://login.other.test")],
            vec![
                url("https://login.mairie360.test"),
                url("https://evil-mairie360.test"),
            ],
        ] {
            assert!(
                WebauthnConfig::new("mairie360.test", Some("  "), origins)
                    .build()
                    .is_err(),
                "an origin outside the relying party must be refused"
            );
        }
    }
}
