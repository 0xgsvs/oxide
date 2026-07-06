use std::env;

use dotenvy::dotenv;

pub struct Config {
    pub database_url: String,
}

/// Loads configuration from environment variables.
///
/// # Panics
///
/// Panics if `DATABASE_URL` is not set.
#[must_use]
pub fn load() -> Config {
    dotenv().ok();

    Config {
        database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
    }
}
