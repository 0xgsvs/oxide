use std::time::Duration;

use redis::{AsyncCommands, aio::MultiplexedConnection};

/// Cache key prefix for task lookups.
const TASK_KEY_PREFIX: &str = "task:";

/// Set a cache entry with a TTL.
pub async fn set_string(con: &mut MultiplexedConnection, key: &str, value: &str, ttl: Duration) {
    let _: Result<(), _> = con.set_ex(key, value, ttl.as_secs()).await;
}

/// Get a cache entry. Returns `None` if missing.
pub async fn get_string(con: &mut MultiplexedConnection, key: &str) -> Option<String> {
    con.get(key).await.ok()
}

/// Delete one or more cache keys.
pub async fn del(con: &mut MultiplexedConnection, keys: &[&str]) {
    let _: Result<(), _> = con.del(keys).await;
}

/// Build a task cache key from its ID.
#[must_use]
pub fn task_key(id: i32) -> String {
    format!("{TASK_KEY_PREFIX}{id}")
}

/// Prefix for task-list cache keys.
pub const TASK_LIST_KEY: &str = "tasks:list";

/// Prefix for per-user list version counter.
const LIST_VERSION_KEY: &str = "tasks:list:version";

/// Bump the list cache version for a user — O(1), atomic, no race.
/// Old list cache entries become unreachable and expire via their own TTL.
pub async fn invalidate_list_cache(con: &mut MultiplexedConnection, user_id: i32) {
    let key = format!("{LIST_VERSION_KEY}:{user_id}");
    let _: Result<(), _> = redis::cmd("INCR").arg(&key).query_async(con).await;
    // TTL so stale version keys don't accumulate.
    let _: Result<(), _> = redis::cmd("EXPIRE")
        .arg(&key)
        .arg(3600i64)
        .query_async(con)
        .await;
}

/// Get the current list cache version for a user (defaults to 0).
pub async fn list_version(con: &mut MultiplexedConnection, user_id: i32) -> i64 {
    let key = format!("{LIST_VERSION_KEY}:{user_id}");
    con.get(&key).await.ok().unwrap_or(0)
}

/// Build a versioned task-list cache key.
pub fn task_list_key(user_id: i32, version: i64, limit: i64, offset: i64) -> String {
    format!("{TASK_LIST_KEY}:{user_id}:{version}:{limit}:{offset}")
}
