use std::env;

use dotenvy::dotenv;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub redis_url: String,
}

/// Loads configuration from environment variables.
///
/// # Panics
///
/// Panics if `DATABASE_URL`, `JWT_SECRET`, or `REDIS_URL` is not set.
#[must_use]
pub fn load() -> Config {
    dotenv().ok();

    Config {
        database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        redis_url: env::var("REDIS_URL").expect("REDIS_URL must be set"),
    }
}
