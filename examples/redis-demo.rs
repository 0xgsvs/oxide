//! Redis basics — the API you'll use in your project.
//! Run: cargo run --example redis-demo
//! Requires: docker compose up -d (redis already running)

use std::time::Duration;

use redis::{AsyncCommands, aio::MultiplexedConnection};

#[tokio::main(flavor = "current_thread")]
async fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379")?;
    let mut con: MultiplexedConnection = client.get_multiplexed_async_connection().await?;
    println!("=== Connected to Redis ===\n");

    // =====================================================================
    // 1. STRINGS
    // =====================================================================
    println!("--- Strings ---");

    // Low-level cmd API — fully explicit about types
    redis::cmd("SET")
        .arg("my_key")
        .arg("hello redis")
        .query_async::<()>(&mut con) // <()> = "no meaningful return"
        .await?;

    let val: String = redis::cmd("GET")
        .arg("my_key")
        .query_async(&mut con)
        .await?;
    println!("  GET my_key: {val}");

    // High-level commands — need explicit type annotation
    // If the return value isn't used: annotate with : ()
    con.set::<_, _, ()>("another_key", 42i32).await;
    let num: i32 = con.get("another_key").await?;
    println!("  GET another_key: {num}");

    // SET with expiry — discard result with semicolon tricks
    let _: () = con.set_ex("temp_key", "will expire", 10u64).await?;
    let ttl: i64 = con.ttl("temp_key").await?;
    println!("  TTL of temp_key: {ttl}s");

    // SET NX — returns bool (true if set, false if exists)
    let set: bool = con.set_nx("my_key", "already exists").await?;
    println!("  SET NX (should be false): {set}");

    // =====================================================================
    // 2. INCR/DECR — atomic counters
    // =====================================================================
    println!("\n--- Counters ---");
    let count: i64 = con.incr("counter", 1).await?;
    println!("  INCR counter: {count}");
    let count: i64 = con.incr("counter", 5).await?;
    println!("  INCR counter +5: {count}");
    let count: i64 = con.decr("counter", 2).await?;
    println!("  DECR counter -2: {count}");

    // =====================================================================
    // 3. LISTS
    // =====================================================================
    println!("\n--- Lists ---");
    let _: () = con.rpush("my_list", "a").await?;
    let _: () = con.rpush("my_list", "b").await?;
    let _: () = con.rpush("my_list", "c").await?;

    let list: Vec<String> = con.lrange("my_list", 0, -1).await?;
    println!("  LRANGE my_list: {list:?}");
    let popped: Option<String> = con.lpop("my_list", None).await?;
    println!("  LPOP: {popped:?}");
    let len: i64 = con.llen("my_list").await?;
    println!("  LLEN: {len}");

    // =====================================================================
    // 4. HASHES
    // =====================================================================
    println!("\n--- Hashes ---");
    let _: () = con.hset("user:1", "name", "Alice").await?;
    let _: () = con.hset("user:1", "age", 30i32).await?;
    let _: () = con.hset("user:1", "role", "admin").await?;

    let name: String = con.hget("user:1", "name").await?;
    println!("  HGET user:1 name: {name}");
    let fields: std::collections::HashMap<String, String> = con.hgetall("user:1").await?;
    println!("  HGETALL user:1: {fields:?}");
    let _: () = con.hincr("user:1", "login_count", 1).await?;
    let count: i32 = con.hget("user:1", "login_count").await?;
    println!("  Login count: {count}");

    // =====================================================================
    // 5. SETS
    // =====================================================================
    println!("\n--- Sets ---");
    let _: () = con.sadd("tags", "rust").await?;
    let _: () = con.sadd("tags", "async").await?;
    let _: () = con.sadd("tags", "redis").await?;
    let _: () = con.sadd("tags", "rust").await?; // duplicate ignored

    let members: Vec<String> = con.smembers("tags").await?;
    println!("  SMEMBERS tags: {members:?}");
    let is_member: bool = con.sismember("tags", "rust").await?;
    println!("  SISMEMBER rust: {is_member}");

    // =====================================================================
    // 6. SORTED SETS
    // =====================================================================
    println!("\n--- Sorted Sets ---");
    let _: () = con.zadd("leaderboard", "Alice", 100.0).await?;
    let _: () = con.zadd("leaderboard", "Bob", 200.0).await?;
    let _: () = con.zadd("leaderboard", "Charlie", 150.0).await?;

    let top: Vec<String> = con.zrevrange("leaderboard", 0, 1).await?;
    println!("  Top 2: {top:?}");
    let rank: i64 = con.zrevrank("leaderboard", "Alice").await?;
    println!("  Alice's rank: {rank}");

    // =====================================================================
    // 7. PUB/SUB
    // =====================================================================
    println!("\n--- Pub/Sub ---");
    // Subscribe on a separate connection, then publish
    let mut pubsub = client.get_async_pubsub().await?;
    pubsub.subscribe("news").await?;
    tokio::spawn(async move {
        use futures::pin_mut;
        let stream = pubsub.on_message();
        pin_mut!(stream);
        if let Some(msg) = futures::StreamExt::next(&mut stream).await {
            let payload: String = msg.get_payload().unwrap();
            println!("  [subscriber] got: {payload}");
        }
    });

    let _: () = con.publish("news", "hello subscribers!").await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // =====================================================================
    // 8. PIPELINE — batch commands
    // =====================================================================
    println!("\n--- Pipeline ---");
    let mut pipe = redis::pipe();
    pipe.cmd("SET").arg("k1").arg("v1").ignore();
    pipe.cmd("SET").arg("k2").arg("v2").ignore();
    pipe.cmd("GET").arg("k1").ignore();
    let _: () = pipe.query_async(&mut con).await?;
    println!("  Pipeline executed 3 commands");

    // =====================================================================
    // 9. KEY MANAGEMENT
    // =====================================================================
    println!("\n--- Key Management ---");
    let exists: bool = con.exists("my_key").await?;
    println!("  EXISTS my_key: {exists}");

    let _: () = con.expire("my_key", 60).await?;
    let deleted: i64 = con.del("temp_key").await?;
    println!("  DEL temp_key: {deleted}");

    let keys: Vec<String> = con.keys("user:*").await?;
    println!("  KEYS user:*: {keys:?}");

    // =====================================================================
    // CLEANUP
    // =====================================================================
    let _: () = redis::cmd("FLUSHDB").query_async(&mut con).await?;
    println!("\nCleaned up test keys. ✅");

    println!("\n=== Cheat sheet ===");
    println!("  Strings:    SET, GET, SET_EX, SET_NX, TTL");
    println!("  Counters:   INCR, DECR (atomic)");
    println!("  Lists:      LPUSH, RPUSH, LPOP, LRANGE, LLEN");
    println!("  Hashes:     HSET, HGET, HGETALL, HINCR");
    println!("  Sets:       SADD, SMEMBERS, SISMEMBER");
    println!("  Sorted:     ZADD, ZRANGE, ZREVRANK");
    println!("  Pub/Sub:    SUBSCRIBE, PUBLISH");
    println!("  Pipeline:   redis::pipe()");
    println!("  Keys:       EXISTS, DEL, KEYS, EXPIRE");
    // Key pattern for commands that return a value you don't need:
    // let _: () = con.some_command(args).await?;
    Ok(())
}
