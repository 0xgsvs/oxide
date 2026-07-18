//! The tokio API essentials for beginners.
//! Run: cargo run --example tokio-essentials

use tokio::{
    join, select,
    sync::{mpsc, oneshot},
    task,
    time::{Duration, sleep, timeout},
};

// =========================================================================
// 1. THE RUNTIME — how async code actually runs
// =========================================================================
//
// #[tokio::main] is a macro that creates a tokio runtime and runs your fn on it.
// The runtime manages a pool of OS threads + a task scheduler + I/O driver.
//
// #[tokio::main]                → multi-threaded (default)
// #[tokio::main(flavor = "current_thread")]  → single-threaded

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== 1. tokio::spawn — run a task in the background ===");
    // spawn adds a task to the runtime's run queue.
    // Returns JoinHandle — you can .await it to get the result.
    let handle: task::JoinHandle<i32> = tokio::spawn(async {
        sleep(Duration::from_millis(100)).await;
        42
    });
    // Do other work while the spawned task runs...
    let result = handle.await.unwrap();
    println!("  spawned task returned: {result}");

    // spawn_blocking — for CPU-heavy or blocking work
    // Runs on a separate thread pool so it doesn't block async tasks.
    let heavy = task::spawn_blocking(|| {
        // No async here — this runs on a dedicated thread
        let mut sum = 0i64;
        for i in 0..10_000_000 {
            sum += i;
        }
        sum
    });
    println!("  blocking task: {}", heavy.await.unwrap());

    println!("\n=== 2. tokio::time — timers ===");
    // sleep — async version of thread::sleep
    sleep(Duration::from_millis(50)).await;
    println!("  slept 50ms");

    // timeout — if the future doesn't finish in time, returns Err
    let result = timeout(Duration::from_millis(10), async {
        sleep(Duration::from_millis(100)).await;
        "done"
    })
    .await;
    match result {
        Ok(val) => println!("  finished: {val}"),
        Err(_) => println!("  timed out! (as expected)"),
    }

    // interval — runs a task periodically
    println!("  interval (3 ticks):");
    let mut interval = tokio::time::interval(Duration::from_millis(30));
    for i in 0..3 {
        interval.tick().await; // waits for the next tick
        println!("    tick {i}");
    }

    println!("\n=== 3. tokio::sync — channels ===");

    // --- oneshot — send ONE value, exactly once ---
    // Like a Promise/Future pair in other languages
    let (tx, rx) = oneshot::channel::<String>();
    tokio::spawn(async {
        // Do some work...
        sleep(Duration::from_millis(50)).await;
        let _ = tx.send("hello from oneshot".to_string());
    });
    let msg = rx.await.unwrap();
    println!("  oneshot: {msg}");

    // --- mpsc — multi-producer, single-consumer channel ---
    // Like a queue. Many senders, one receiver. Bounded or unbounded.
    let (tx, mut rx) = mpsc::channel::<i32>(4); // buffer of 4 items

    // Spawn two producers
    for id in 0..2 {
        let tx = tx.clone();
        tokio::spawn(async move {
            for i in 0..3 {
                tx.send(i).await.unwrap();
                sleep(Duration::from_millis(10)).await;
            }
        });
    }
    // Drop the original sender so rx stops when all producers are done
    drop(tx);

    // Consumer — receives until all senders are dropped
    let mut count = 0;
    while let Some(val) = rx.recv().await {
        count += 1;
        print!(" {val}");
    }
    println!("\n  mpsc: received {count} messages total");

    // --- watch — one value, many readers, always last value ---
    let (tx, mut rx) = tokio::sync::watch::channel(0i32);
    tokio::spawn(async move {
        for i in 1..=3 {
            tx.send(i).unwrap();
            sleep(Duration::from_millis(30)).await;
        }
    });
    // The receiver always gets the LATEST value
    // First recv() is immediate — gets the current value (0)
    // Subsequent recv() wait for a change
    while rx.changed().await.is_ok() {
        println!("  watch: value changed to {}", *rx.borrow());
    }

    // --- broadcast — one value to MANY receivers ---
    let (tx, mut rx1) = tokio::sync::broadcast::channel::<&str>(16);
    let mut rx2 = tx.subscribe();
    tokio::spawn(async move {
        for msg in ["ping", "pong", "done"] {
            tx.send(msg).unwrap();
            sleep(Duration::from_millis(20)).await;
        }
    });
    // Both receivers get every message
    println!("  broadcast:");
    // Lazily just read first 2 from rx1
    println!("    rx1: {}", rx1.recv().await.unwrap());
    println!("    rx1: {}", rx1.recv().await.unwrap());

    println!("\n=== 4. tokio::sync — Mutex for async ===");
    // Use tokio::sync::Mutex when you hold the lock across .await
    // (parking_lot::Mutex is fine for quick locks without .await)
    let shared = std::sync::Arc::new(tokio::sync::Mutex::new(0i32));
    let mut handles = vec![];
    for _ in 0..5 {
        let lock = std::sync::Arc::clone(&shared);
        handles.push(tokio::spawn(async move {
            let mut val = lock.lock().await;
            *val += 1;
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    println!("  mutex: final value = {}", *shared.lock().await);

    println!("\n=== 5. tokio macros ===");

    // join! — wait for ALL futures, run them concurrently
    // We already covered this. It returns Pending only when ALL are Pending.
    let (a, b) = join!(
        async {
            sleep(Duration::from_millis(30)).await;
            "slow"
        },
        async { "fast" }, // no .await → Ready immediately
    );
    println!("  join!: {a}, {b}");

    // select! — race futures, first Ready wins, others are DROPPED
    select! {
        _ = sleep(Duration::from_millis(100)) => {
            println!("  select!: timeout won (shouldn't happen)");
        }
        result = async { sleep(Duration::from_millis(10)).await; 42 } => {
            println!("  select!: fast future won with {result}");
        }
    }

    println!("\n=== 6. tokio::net — networking ===");
    // Quick echo server/client example
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0") // port 0 = OS assigns
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn a client that connects and sends a message
    tokio::spawn(async move {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        use tokio::io::AsyncWriteExt;
        stream.write_all(b"hello server!").await.unwrap();
    });

    // Accept one connection and read
    let (mut stream, _) = listener.accept().await.unwrap();
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf).await.unwrap();
    println!("  tcp: received: {}", String::from_utf8_lossy(&buf[..n]));

    println!("\n=== Summary of tokio's essential API ===");
    println!("  Runtime:      #[tokio::main], tokio::spawn, spawn_blocking");
    println!("  Time:         sleep, timeout, interval");
    println!("  Sync:         oneshot, mpsc, watch, broadcast, Mutex");
    println!("  Macros:       join!, select!, try_join!");
    println!("  Net:          TcpListener, TcpStream (UdpSocket too)");
    println!("  I/O:          AsyncReadExt, AsyncWriteExt (traits)");
}
