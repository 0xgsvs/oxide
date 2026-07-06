use sqlx::PgPool;

/// Creates a PostgreSQL connection pool.
///
/// # Panics
///
/// Panics if the database connection cannot be established.
pub async fn create_pool(database_url: &str) -> PgPool {
    PgPool::connect(database_url)
        .await
        .expect("Failed to connect to database")
}
