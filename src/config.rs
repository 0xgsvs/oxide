use std::env;

use dotenvy::dotenv;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
}

/// Loads configuration from environment variables.
///
/// # Panics
///
/// Panics if `DATABASE_URL` or `JWT_SECRET` is not set.
#[must_use]
pub fn load() -> Config {
    dotenv().ok();

    Config {
        database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
    }
}
