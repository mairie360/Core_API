//! Modules a role opens (MAIR-916): the entries of the shared navigation of lib-components, stored in
//! `role_modules` (Database `mair-916-01`). The `Admin` role always keeps `admin` (a trigger refuses
//! to take it away, `23514`).

use crate::endpoints::validation::ValidationError;

/// Every module of the navigation, in the order of the navigation.
pub const MODULES: [&str; 7] = [
    "dashboard",
    "projects",
    "calendar",
    "messages",
    "training",
    "admin",
    "settings",
];

/// Modules of a role created without any: the dashboard (MAIR-916 F16).
pub const DEFAULT_MODULES: [&str; 1] = ["dashboard"];

/// The module the `Admin` role cannot lose.
pub const ADMIN_MODULE: &str = "admin";

/// Checks a list of modules: known ids only, no duplicate.
///
/// # Errors
/// A `400` naming `field` for an unknown or repeated module.
pub fn check_modules(field: &str, modules: &[String]) -> Result<(), ValidationError> {
    for (index, module) in modules.iter().enumerate() {
        if !MODULES.contains(&module.as_str()) {
            return Err(ValidationError::new(
                field,
                "must only hold dashboard, projects, calendar, messages, training, admin or settings",
            ));
        }
        if modules[..index].contains(module) {
            return Err(ValidationError::new(field, "must not repeat a module"));
        }
    }
    Ok(())
}

/// The modules sorted in the order of the navigation, joined with commas for one text parameter
/// (`QueryParam` has no array variant; every value is a known id, so none holds a comma).
#[must_use]
pub fn joined(modules: &[String]) -> String {
    MODULES
        .iter()
        .filter(|module| modules.iter().any(|m| m == *module))
        .copied()
        .collect::<Vec<_>>()
        .join(",")
}

/// Orders modules read from the database like the navigation.
#[must_use]
pub fn ordered(modules: Vec<String>) -> Vec<String> {
    MODULES
        .iter()
        .filter(|module| modules.iter().any(|m| m == *module))
        .map(|module| (*module).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_modules_and_refuses_others() {
        let ok = vec!["projects".to_string(), "dashboard".to_string()];
        assert!(check_modules("modules", &ok).is_ok());
        assert!(check_modules("modules", &["files".to_string()]).is_err());
        assert!(check_modules("modules", &["admin".to_string(), "admin".to_string()]).is_err());
        assert!(check_modules("modules", &[]).is_ok());
    }

    #[test]
    fn joins_and_orders_like_the_navigation() {
        let modules = vec!["settings".to_string(), "dashboard".to_string()];
        assert_eq!(joined(&modules), "dashboard,settings");
        assert_eq!(ordered(modules), vec!["dashboard", "settings"]);
        assert_eq!(joined(&[]), "");
    }
}
