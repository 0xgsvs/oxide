use std::env::var;

use dotenvy::dotenv;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub redis_url: String,
    pub addr: String,
}

/// Loads configuration from environment variables.
///
/// # Panics
///
/// Panics if `DATABASE_URL`, `JWT_SECRET`, `REDIS_URL`, or `ADDR` is not set.
#[must_use]
pub fn load() -> Config {
    dotenv().ok();

    Config {
        database_url: var("DATABASE_URL").expect("DATABASE_URL must be set"),
        jwt_secret: var("JWT_SECRET").expect("JWT_SECRET must be set"),
        redis_url: var("REDIS_URL").expect("REDIS_URL must be set"),
        addr: var("ADDR").expect("ADDR must be set"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Characterization test: load() parses the four expected env vars.
    ///
    /// Writes env vars, calls load(), asserts all fields match.
    /// Tests must run serially — env vars are process-global.
    #[test]
    fn load_returns_config_with_all_env_vars_set() {
        // ponytail: set_var/remove_var are unsafe in edition 2024. Safe here
        // because tests are single-threaded and no concurrent var() call.
        unsafe {
            std::env::set_var("DATABASE_URL", "postgres://test");
            std::env::set_var("JWT_SECRET", "my-secret");
            std::env::set_var("REDIS_URL", "redis://test");
            std::env::set_var("ADDR", "0.0.0.0:8080");
        }

        let cfg = load();

        assert_eq!(cfg.database_url, "postgres://test");
        assert_eq!(cfg.jwt_secret, "my-secret");
        assert_eq!(cfg.redis_url, "redis://test");
        assert_eq!(cfg.addr, "0.0.0.0:8080");

        unsafe {
            std::env::remove_var("DATABASE_URL");
            std::env::remove_var("JWT_SECRET");
            std::env::remove_var("REDIS_URL");
            std::env::remove_var("ADDR");
        }
    }
}
