use std::time::Duration;

use redis::{AsyncCommands, aio::MultiplexedConnection};

/// Cache key prefix for task lookups.
const TASK_KEY_PREFIX: &str = "task:";

/// Set a cache entry with a TTL.
pub async fn set_string(con: &mut MultiplexedConnection, key: &str, value: &str, ttl: Duration) {
    let _: Result<(), _> = redis::cmd("SET")
        .arg(key)
        .arg(value)
        .arg("EX")
        .arg(ttl.as_secs())
        .query_async(con)
        .await;
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

/// Build the task-list cache key.
pub const TASK_LIST_KEY: &str = "tasks:list";

/// Delete all keys matching a prefix using SCAN.
/// ponytail: SCAN is safe for concurrent use but not atomic.
/// A concurrent write could sneak in between SCAN and DEL.
/// Fine for dev — production would use a versioned key pattern.
pub async fn del_by_prefix(con: &mut MultiplexedConnection, prefix: &str) {
    let pattern = format!("{prefix}:*");
    let mut cursor = 0u64;
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(100i32)
            .query_async(con)
            .await
            .unwrap_or((0, vec![]));
        if !keys.is_empty() {
            let _: Result<(), _> = con
                .del(&keys.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .await;
        }
        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }
}
