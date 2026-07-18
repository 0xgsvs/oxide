//! Single-threaded async demo showing exactly what happens with .await
//! Run: cargo run --example async-await-demo

use tokio::time::{Duration, sleep};

// =========================================================================
// .await is NOT blocking. It's YIELDING.
// =========================================================================
//
// Blocking = thread can't do ANYTHING else (stuck on CPU or OS sleep)
// Yielding = task says "I'm waiting, let another task use this thread"
//
// On a single thread with tokio::spawn:
//   Task A hits .await → Pending → executor runs Task B
//   Task B hits .await → Pending → executor runs Task A (woken up)
//
// Without tokio::spawn:
//   One task, one thread → .await yields but no other task to run
//   → executor parks the thread → waker fires → thread unparked → continues

async fn task_a() {
    println!("  [A] start");
    sleep(Duration::from_millis(100)).await;
    println!("  [A] after 100ms sleep");
    sleep(Duration::from_millis(100)).await;
    println!("  [A] done");
}

async fn task_b() {
    println!("  [B] start");
    sleep(Duration::from_millis(50)).await;
    println!("  [B] after 50ms sleep");
    sleep(Duration::from_millis(50)).await;
    println!("  [B] done");
}

#[tokio::main(flavor = "current_thread")] // SINGLE thread
async fn main() {
    println!("=== Scenario 1: Sequential .await (no spawn) ===");
    println!("Only ONE task, one thread. .await still yields but nothing else runs.\n");
    let start = std::time::Instant::now();
    task_a().await;
    println!("({:?} total)\n", start.elapsed());

    println!("=== Scenario 2: Concurrent with tokio::spawn (same single thread) ===");
    println!("Two tasks. When one .await yields, the OTHER runs on the SAME thread.\n");

    let start = std::time::Instant::now();
    let h1 = tokio::spawn(task_a());
    let h2 = tokio::spawn(task_b());

    // Both tasks are queued on the same thread's executor.
    // Timeline on the single thread:
    //   1. Executor polls task_a → hits sleep(100) → Pending
    //   2. Executor polls task_b → hits sleep(50) → Pending
    //   3. 50ms later: task_b's timer fires → poll task_b → Ready
    //   4. Executor polls task_b → hits sleep(50) → Pending
    //   5. 100ms later: task_a's timer fires → poll task_a → Ready
    //   6. Executor polls task_a → hits sleep(100) → Pending
    //   7. 50ms later: task_b's timer fires → poll task_b → Ready → done
    //   8. 100ms later: task_a's timer fires → poll task_a → Ready → done

    h1.await.unwrap();
    h2.await.unwrap();
    println!("({:?} total — tasks OVERLAPPED!)\n", start.elapsed());

    println!("=== Scenario 3: .await on the spawn handle ===");
    println!("h1.await waits for task_a. But task_b runs WHILE we wait.\n");

    let start = std::time::Instant::now();
    let h1 = tokio::spawn(task_a());
    let h2 = tokio::spawn(task_b());

    // This .await waits for task_a specifically.
    // Meanwhile, the executor runs task_b whenever task_a yields.
    h1.await.unwrap();
    println!("task_a done, task_b might still be running...");
    h2.await.unwrap();
    println!("({:?} total)\n", start.elapsed());

    println!("=== Scenario 4: Blocking BAD ===");
    println!("std::thread::sleep BLOCKS the thread. Nothing else can run.\n");

    let start = std::time::Instant::now();
    let h = tokio::spawn(async {
        // This is async, so it yields properly
        sleep(Duration::from_millis(50)).await;
        println!("  [async] ran after 50ms");
    });

    // This is BAD in async code — blocks the ENTIRE thread
    std::thread::sleep(Duration::from_millis(100));
    println!("  [main] blocked thread for 100ms");

    h.await.unwrap();
    println!(
        "({:?} total — notice async task finished AFTER main blocked!)\n",
        start.elapsed()
    );
    println!("With std::thread::sleep, the thread can't run the spawned task.");
    println!("With tokio::time::sleep, the thread runs OTHER tasks while waiting.");
}
