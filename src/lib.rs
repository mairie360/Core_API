pub mod register;
pub mod database;

pub fn get_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn get_critical_env_var(name: &str) -> String {
    match std::env::var(name) {
        Ok(val) => val,
        Err(_) => {
            eprintln!("Error: Environment variable '{}' is not set.", name);
            std::process::exit(1);
        },
    }
}