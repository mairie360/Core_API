use std::env;
use once_cell::sync::Lazy;
use core_api::{get_critical_env_var, get_env_var};

static INIT: Lazy<()> = Lazy::new(|| {
    // This code runs ONCE before any test
    env::set_var("MY_KEY", "global_value");
    println!("Global setup done");
});

fn setup() {
    // Force INIT to run
    Lazy::force(&INIT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_var() {
        setup();
        assert_eq!(get_env_var("MY_KEY").unwrap(), "global_value");
    }

    #[test]
    fn test_get_env_var_not_found() {
        setup();
        assert!(get_env_var("NON_EXISTENT_KEY").is_none());
    }

    #[test]
    fn test_get_critical_env_var() {
        setup();
        assert_eq!(get_critical_env_var("MY_KEY"), "global_value");
    }

    #[test]
    #[should_panic(expected = "Critical environment variable 'NON_EXISTENT_KEY' is not set")]
    fn test_get_critical_env_var_not_found() {
        setup();
        get_critical_env_var("NON_EXISTENT_KEY");
    }
}
