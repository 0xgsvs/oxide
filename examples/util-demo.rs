//! tokio-util + rayon basics.
//! Run: cargo run --example util-demo

// =========================================================================
// A. tokio-util — what it provides
// =========================================================================
//
// tokio-util is the "extras" crate for tokio. Things that are useful but
// not essential enough for the core tokio crate.
//
// ┌────────────────────────────────────────────────────────────────────┐
// │ Module        │ What it gives you                                 │
// ├────────────────────────────────────────────────────────────────────┤
// │ codec         │ Framing: turn AsyncRead/AsyncWrite into            │
// │               │ Stream/Sink of structured messages (length-prefix, │
// │               │ lines, etc.) — for custom protocols               │
// ├────────────────────────────────────────────────────────────────────┤
// │ compat        │ Bridges tokio::io and futures-rs AsyncRead/Write   │
// │               │ (use when a library uses futures traits)           │
// ├────────────────────────────────────────────────────────────────────┤
// │ future        │ TryStreamExt, StreamExt, etc. — extra combinators  │
// │               │ for futures and streams                            │
// ├────────────────────────────────────────────────────────────────────┤
// │ io            │ ReaderStream, StreamReader — bridge between        │
// │               │ byte streams and async streams                    │
// ├────────────────────────────────────────────────────────────────────┤
// │ sync          │ CancellationToken, PollSemaphore, etc.             │
// │               │ Extra sync primitives                              │
// ├────────────────────────────────────────────────────────────────────┤
// │ task          │ spawn_pinned — run a !Send future on the main      │
// │               │ thread. LocalPoolHandle — pool of single-thread    │
// │               │ executors for !Send work.                          │
// ├────────────────────────────────────────────────────────────────────┤
// │ time          │ DelayQueue — futures that complete after a delay   │
// │               │ (like sleep but with a key to cancel/poll)         │
// ├────────────────────────────────────────────────────────────────────┤
// │ udp/net       │ UdpFramed — frame UDP into messages                │
// │ codec+net     │                                                      │
// └────────────────────────────────────────────────────────────────────┘
//
// When you need it:
//   - Building a custom TCP protocol? → codec module
//   - Need to spawn !Send futures? → task::spawn_pinned
//   - Bridging tokio ↔ futures crate? → compat
//   - Want to cancel a group of tasks? → sync::CancellationToken

// =========================================================================
// B. Rayon — data parallelism for CPU work
// =========================================================================
//
// Rayon is for CPU-bound PARALLELISM (multiple cores, same task).
// Tokio is for I/O-bound CONCURRENCY (many tasks waiting on I/O).
//
// ┌────────────────────────────────────────────────────────────────────┐
// │ Rayon API            │ What it does                               │
// ├────────────────────────────────────────────────────────────────────┤
// │ vec.par_iter()       │ Parallel iterator — splits work across CPUs │
// │ vec.par_sort()       │ Parallel sort                              │
// │ ray.join(|| ..., || ...) │ Run TWO closures in parallel, wait both   │
// │ ray::scope(\|s\| ...)    │ Scoped spawn — tasks can borrow from stack │
// │ ray::spawn(\|\| ...)     │ Fire-and-forget on rayon thread pool      │
// │ ThreadPoolBuilder    │ Configure number of threads, etc.           │
// └────────────────────────────────────────────────────────────────────┘

fn rayon_basics() {
    use rayon::prelude::*;

    // --- Parallel iterator ---
    // Split a Vec across all CPU cores
    let nums: Vec<i64> = (0..10_000_000).collect();
    let sum: i64 = nums.par_iter().sum(); // auto-parallelized!
    println!("  par_iter sum: {sum}");

    let max = nums.par_iter().max().unwrap();
    println!("  par_iter max: {max}");

    // Parallel sort
    let mut words = vec!["banana", "apple", "cherry", "date", "elderberry"];
    words.par_sort(); // stable parallel sort
    println!("  par_sort: {words:?}");

    // --- join() — run two things in parallel, wait for both ---
    let (a, b) = rayon::join(
        || {
            let mut sum = 0i64;
            for i in 0..5_000_000 {
                sum += i;
            }
            sum
        },
        || {
            let mut sum = 0i64;
            for i in 5_000_000..10_000_000 {
                sum += i;
            }
            sum
        },
    );
    println!("  join total: {} (a={a}, b={b})", a + b);

    // --- scope() — spawn scoped tasks that borrow from the stack ---
    let data = vec![1, 2, 3, 4, 5];
    let results = std::sync::Mutex::new(vec![]);
    // Use a reference to avoid moving results into the move closure
    let results_ref = &results;
    rayon::scope(|s| {
        for &x in &data {
            s.spawn(move |_| {
                let doubled = x * 2;
                results_ref.lock().unwrap().push(doubled);
            });
        }
    });
    println!("  scope: {:?}", results.lock().unwrap());
}

// =========================================================================
// Using rayon WITH tokio — the key question
// =========================================================================
//
// YES, they work together perfectly. But you MUST not block tokio's threads.
// Pattern: wrap rayon work in spawn_blocking so it runs on a SEPARATE thread.
//
// Why separate?
//   - tokio's threads run async tasks. If you block them with rayon CPU work,
//     no other async task can run.
//   - spawn_blocking moves the work to a dedicated thread pool.
//   - rayon's thread pool is ENTIRELY separate from tokio's.
//     They don't conflict — they're different OS threads.

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== tokio-util modules ===");
    println!("  codec  — frame TCP streams into messages");
    println!("  compat — bridge tokio and futures-rs traits");
    println!("  future — StreamExt, TryStreamExt combinators");
    println!("  io     — bridge byte streams <-> async streams");
    println!("  sync   — CancellationToken, PollSemaphore");
    println!("  task   — spawn_pinned (!Send futures)");
    println!("  time   — DelayQueue");

    println!("\n=== Rayon basics ===");
    rayon_basics();

    println!("\n=== Rayon + Tokio together ===");
    // Pattern 1: spawn_blocking wrapping rayon work
    let start = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(|| {
        // This runs on tokio's BLOCKING thread pool (separate from async threads)
        use rayon::prelude::*;
        let v: Vec<i64> = (0..10_000_000).collect();
        v.par_iter().sum::<i64>()
    })
    .await
    .unwrap();
    println!("  spawn_blocking + rayon: {result} ({:?})", start.elapsed());

    // Pattern 2: oneshot + rayon::spawn (more explicit)
    let start = std::time::Instant::now();
    let (tx, rx) = tokio::sync::oneshot::channel();
    rayon::spawn(move || {
        // This runs on rayon's thread pool
        use rayon::prelude::*;
        let v: Vec<i64> = (0..10_000_000).collect();
        let sum = v.par_iter().sum::<i64>();
        let _ = tx.send(sum); // send result back to async world
    });
    let result = rx.await.unwrap();
    println!("  oneshot + rayon::spawn: {result} ({:?})", start.elapsed());

    // Pattern 3: Parallel work with async coordination
    println!("\n=== Real-world: parallel CPU work in an API handler ===");
    println!("  In your axum handler:");
    println!("  async fn analyze(Json(req): Json<Payload>) -> Result<Json<Report>, AppError> {{");
    println!("      let report = tokio::task::spawn_blocking(move || {{");
    println!("          // Heavy CPU work — uses ALL cores via rayon");
    println!("          req.data.par_iter()...");
    println!("      }}).await.unwrap();");
    println!("      Ok(Json(report))");
    println!("  }}");

    println!("\n=== When to use which ===");
    println!("  Tokio:         I/O, networking, waiting (your API, DB queries)");
    println!("  Rayon:         CPU crunching (image processing, sorting, parsing)");
    println!("  spawn_blocking: Bridge between them (rayon inside this)");
}
