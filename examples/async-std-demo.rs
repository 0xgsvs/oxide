//! Run: cargo run --example async-std-demo
//!
//! What does `std` provide for async? And what does tokio add on top?
//! This file builds async from the ground up — no tokio, just std.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWakerVTable, Waker},
    time::{Duration, Instant},
};

// =========================================================================
// 1. The core: `Future` trait — just 1 method, 1 enum
// =========================================================================
//
// pub trait Future {
//     type Output;
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
// }
//
// pub enum Poll<T> {
//     Ready(T),
//     Pending,
// }
//
// That's it. The entire async system is built on these 15 lines.
//
// "poll" = "are you done yet?"
//   Ready(value) = "yes, here's the result"
//   Pending     = "not yet, wake me when there's progress"

// =========================================================================
// 2. A custom Future from scratch (no async sugar)
// =========================================================================

/// A future that completes after `duration` has elapsed.
/// No async/await, no tokio — pure std.
struct Sleep {
    wake_time: Instant,
}

impl Sleep {
    fn new(duration: Duration) -> Self {
        Self {
            wake_time: Instant::now() + duration,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.wake_time {
            println!("  Sleep: time's up! Returning Ready");
            Poll::Ready(())
        } else {
            // Not ready yet. Tell the runtime to wake us when it's time.
            let waker = cx.waker().clone();
            let remaining = self.wake_time - Instant::now();
            println!("  Sleep: not yet ({remaining:?} left), scheduling wakeup");

            // In a real executor, you'd register the waker with epoll/kqueue/IOCP.
            // Here we simulate by spawning a thread that wakes us later.
            std::thread::spawn(move || {
                std::thread::sleep(remaining);
                waker.wake(); // <-- THIS is what makes the executor poll again
            });

            Poll::Pending
        }
    }
}

// =========================================================================
// 3. What `async fn` desugars to
// =========================================================================

// When you write:
//   async fn hello() -> String {
//       "hello".to_string()
//   }
//
// The compiler generates something like:
//   fn hello() -> HelloFuture { HelloFuture }
//   struct HelloFuture;
//   impl Future for HelloFuture {
//       type Output = String;
//       fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<String> {
//           Poll::Ready("hello".to_string())
//       }
//   }

async fn hello() -> String {
    "hello".to_string()
}

// When you write `.await`, the compiler generates a state machine.
// Each `.await` point becomes a state in an enum:
//
//   async fn greet() -> String {
//       let first = hello().await;     // state 0 → poll inner future
//       let second = hello().await;    // state 1 → poll inner future again
//       format!("{first} {second}")    // state 2 → ready
//   }
//
// Desugars to (roughly):
//   enum GreetFuture {
//       State0 { first_fut: HelloFuture },
//       State1 { first: String, second_fut: HelloFuture },
//       State2,
//   }

async fn greet() -> String {
    let first = hello().await;
    let second = hello().await;
    format!("{first} {second}")
}

// =========================================================================
// 4. What std gives you vs what tokio gives you
// =========================================================================

// "But std::future::Future doesn't have a timer! How do I sleep?"
//
// STD provides:
//   ✅ Future trait + Poll + Context + Waker   — the *protocol*
//   ✅ Pin — needed because async state machines are self-referential
//   ✅ async/await keywords — syntactic sugar
//   ✅ IntoFuture — so you can pass anything that can become a Future
//
// STD does NOT provide:
//   ❌ An executor (no "block_on", no "tokio::spawn")
//   ❌ I/O (no async read/write)
//   ❌ Timers (no sleep)
//   ❌ Networking (no TcpListener)
//   ❌ Channels (no mpsc)
//
// TOKIO provides:
//   ✅ An executor (multi-threaded work-stealing)
//   ✅ Async I/O (epoll/kqueue/IOCP under the hood)
//   ✅ Timers (tokio::time::sleep)
//   ✅ Networking (tokio::net::TcpListener)
//   ✅ Channels, sync primitives, fs, signal handling
//   ✅ spawn — run tasks concurrently on the same thread pool
//
// The executor is the piece that:
//   1. Calls poll() on your futures
//   2. Sleeps when all futures return Pending
//   3. Wakes the right future when Waker::wake() is called
//   4. Distributes work across threads

// =========================================================================
// 5. A minimal executor — to show how poll() actually gets called
// =========================================================================

/// A single-threaded executor that blocks on one future.
/// This is what `tokio::main` / `futures::executor::block_on` does internally.
fn block_on<F: Future>(mut fut: F) -> F::Output {
    let main_thread = std::thread::current();

    // Build a waker that calls unpark on the main thread.
    let waker = thread_unpark_waker(main_thread);
    let mut cx = Context::from_waker(&waker);

    // Pin the future on the stack
    // SAFETY: fut is owned and won't move while pinned
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };

    println!("  Executor: entering poll loop");
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(val) => {
                println!("  Executor: got Ready!");
                return val;
            }
            Poll::Pending => {
                println!("  Executor: got Pending, parking...");
                std::thread::park(); // sleeps until unparked
                println!("  Executor: woken up, polling again");
            }
        }
    }
}

/// Build a Waker whose `wake()` calls `thread.unpark()`.
fn thread_unpark_waker(thread: std::thread::Thread) -> Waker {
    use std::task::RawWaker;

    // SAFETY: Thread never dropped before used — we own the Box.
    let ptr = Box::into_raw(Box::new(thread));

    unsafe fn clone(ptr: *const ()) -> RawWaker {
        let t = unsafe { &*(ptr as *const std::thread::Thread) };
        RawWaker::new(Box::into_raw(Box::new(t.clone())) as *const (), &VTABLE)
    }
    unsafe fn wake(ptr: *const ()) {
        let t = unsafe { Box::from_raw(ptr as *mut std::thread::Thread) };
        t.unpark();
    }
    unsafe fn wake_by_ref(ptr: *const ()) {
        let t = unsafe { &*(ptr as *const std::thread::Thread) };
        t.unpark();
    }
    unsafe fn drop_data(ptr: *const ()) {
        unsafe { drop(Box::from_raw(ptr as *mut std::thread::Thread)) };
    }

    const VTABLE: std::task::RawWakerVTable =
        RawWakerVTable::new(clone, wake, wake_by_ref, drop_data);

    unsafe { Waker::from_raw(RawWaker::new(ptr as *const (), &VTABLE)) }
}

// =========================================================================
// 6. Pin — why it exists
// =========================================================================
//
// Async state machines contain self-references:
//
//   async fn example() {
//       let x = String::from("hello");
//       let y = &x;              // y points to x's stack location
//       some_other_fut().await;  // <-- if the whole struct moved, x moves,
//                                //     but y still points at the OLD address
//       println!("{y}");         // use-after-free!
//     -------------------------
//     | State 0:              |
//     |   x: String           |  <- on the HEAP or pinned so it doesn't move
//     |   y: &String          |  <- points to x's address
//     |   other_fut: ...      |
//     -------------------------
//
// Pin guarantees the value won't move after it's pinned.
// Box::pin(x) or std::pin::pin!(x) pins it to the stack.
//
// That's why Future::poll takes Pin<&mut Self> — the future must stay put.

// =========================================================================
// 7. The golden rule of async
// =========================================================================
//
// #1: Never block the thread in async code
//     ❌ std::thread::sleep(Duration::from_secs(1))
//     ✅ tokio::time::sleep(Duration::from_secs(1)).await
//     If you block, the ENTIRE thread pool can't run other tasks.
//
// #2: .await yields control back to the executor
//     When you hit .await, your task may be suspended and another task runs.
//     Between .await points, your code runs synchronously.
//
// #3: std::sync::Mutex is dangerous in async code
//     If the lock is contended, the thread blocks. Use tokio::sync::Mutex
//     for locks held across .await, or parking_lot::Mutex for quick locks
//     that don't cross .await.
//
// #4: Computation-heavy work still blocks
//     for i in 0..1_000_000 { heavy_calc(i); } ← blocks the thread
//     Use tokio::task::spawn_blocking for CPU work.

fn main() {
    println!("=== 1. Hello, async (no sugar) ===");
    let sleep = Sleep::new(Duration::from_millis(200));
    block_on(sleep);

    println!("\n=== 2. async fn + .await ===");
    let result = block_on(greet());
    println!("Result: {result}");

    println!("\n=== 3. IntoFuture — anything that can become a Future ===");
    // .await works on anything implementing IntoFuture
    let result: String = block_on(async { hello().await });
    println!("IntoFuture: {result}");

    println!("\n=== Recap ===");
    println!("std gives: Future trait   ← the *what*");
    println!("tokio gives: executor+IO  ← the *how*");
    println!("Key insight: poll returns Ready or Pending.");
    println!("Pending means: call Waker::wake() when there's progress.");
    println!("Pin means: the future can't move (needed for self-references).");
}
