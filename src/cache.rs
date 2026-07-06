use redis::AsyncCommands;
use std::time::Duration;

/// Cache key prefix for task lookups.
const TASK_KEY_PREFIX: &str = "task:";

/// Set a cache entry with a TTL.
pub async fn set_string(
    con: &mut redis::aio::MultiplexedConnection,
    key: &str,
    value: &str,
    ttl: Duration,
) {
    let mut con = con.clone();
    let _: Result<(), _> = redis::cmd("SET")
        .arg(key)
        .arg(value)
        .arg("EX")
        .arg(ttl.as_secs())
        .query_async(&mut con)
        .await;
}

/// Get a cache entry. Returns `None` if missing.
pub async fn get_string(
    con: &mut redis::aio::MultiplexedConnection,
    key: &str,
) -> Option<String> {
    let mut con = con.clone();
    con.get(key).await.ok()
}

/// Delete one or more cache keys.
pub async fn del(con: &mut redis::aio::MultiplexedConnection, keys: &[&str]) {
    let mut con = con.clone();
    let _: Result<(), _> = con.del(keys).await;
}

/// Build a task cache key from its ID.
pub fn task_key(id: i32) -> String {
    format!("{TASK_KEY_PREFIX}{id}")
}

/// Build the task-list cache key.
pub const TASK_LIST_KEY: &str = "tasks:list";
