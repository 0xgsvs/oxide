//! Run: cargo run --example async-vs-blocking

use tokio::time::{Duration, sleep, timeout};

// =========================================================================
// ".await without spawn parks the thread — same as blocking?"
// =========================================================================
//
// At the OS level: YES, the thread goes to sleep in both cases.
// But the MECHANISM is different, and that difference matters.
//
// std::thread::sleep(100ms):
//   1. Thread says "wake me in 100ms" → kernel puts thread to sleep
//   2. Timer interrupt at 100ms → kernel wakes thread
//   3. Thread continues. ❌ Cannot be interrupted early (no waker).
//
// tokio::time::sleep(100ms).await:
//   1. Creates a timer, registers a Waker
//   2. Returns Pending → executor parks the thread
//   3. Timer fires → Waker::wake() → executor unparks thread
//   4. Polls again → Ready → continues
//   ✅ CAN be interrupted early — drop the future or use select!
//
// The .await path goes through the EXECUTOR, which means the executor
// can INTERRUPT the wait. Blocking bypasses the executor entirely.

async fn async_sleep() {
    println!("  async_sleep: waiting 100ms...");
    sleep(Duration::from_millis(100)).await;
    println!("  async_sleep: done");
}

async fn async_slow_work() -> &'static str {
    println!("  async_slow_work: starting (takes 200ms)...");
    sleep(Duration::from_millis(200)).await;
    println!("  async_slow_work: done");
    "result"
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== 1. Blocking can't be interrupted ===");
    // std::thread::sleep for 1 second — no way to cancel it early
    // Even though we're "done" at 500ms, the thread is stuck
    println!("  You: \"I want to wait at most 500ms\"");
    println!("  But std::thread::sleep takes a fixed duration.");
    println!("  With .await, you wrap it in timeout():\n");

    let result = timeout(Duration::from_millis(500), async_slow_work()).await;
    match result {
        Ok(val) => println!("  Got result: {val}"),
        Err(_) => println!("  Timed out after 500ms"),
    }

    println!("\n=== 2. select! — race two waits, cancel the loser ===");
    // With blocking, you can't "race" two operations.
    // With .await + select!, whichever finishes first WINS, the other is DROPPED.
    println!("  Race: a 100ms sleep vs immediate result\n");

    tokio::select! {
        _ = async_sleep() => {
            println!("  (main) sleep finished first — but we expected immediate!?)");
        }
        result = async {
            println!("  (race) doing a quick sync calculation...");
            // No .await here — runs immediately
            42
        } => {
            println!("  (main) quick calc won: {result}");
        }
    }

    println!("\n=== 3. Why this matters: request timeout in your API ===");
    println!("  In your axum handler, if a DB query hangs:");
    println!("  ❌ std::thread::sleep — blocks the thread, other requests wait");
    println!("  ✅ timeout + .await — only this request times out, thread is free\n");

    println!("=== Summary ===");
    println!("Behavior           | std::thread::sleep | .await + sleep");
    println!("-------------------|--------------------|-----------------");
    println!("Thread sleeps?     | Yes                | Yes");
    println!("Cancellable?       | No                 | Yes (drop/select)");
    println!("Timeout-able?      | No                 | Yes (timeout())");
    println!("Other tasks run?   | No                 | Yes (with spawn)");
    println!("Stack usage        | 8MB OS thread      | ~100 bytes state");
    println!("");
    println!("The difference isn't what the THREAD does (sleeps in both).");
    println!("It's that .await goes through the EXECUTOR, which gives you");
    println!("cancellation, timeout, select, and composition for FREE.");
}
