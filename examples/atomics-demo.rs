//! Run: cargo run --example atomics-demo

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

// ===== What are atomics? =====
//
// Normal reads/writes to a variable like `x += 1` are NOT thread-safe:
//   thread A reads x (5)
//   thread B reads x (5)
//   thread A writes 6
//   thread B writes 6  ← wrong! x should be 7
//
// Atomics give you operations the CPU guarantees are *indivisible*:
//   x.fetch_add(1, ...) — read-modify-write as one step, no thread can interleave
//
// ===== Key methods (same on all Atomic types) =====

fn atomic_methods() {
    let counter = AtomicI32::new(0);

    // store / load — basic thread-safe read/write
    counter.store(42, Ordering::Relaxed);
    println!("load: {}", counter.load(Ordering::Relaxed)); // 42

    // fetch_add / fetch_sub — atomic += / -=
    counter.fetch_add(10, Ordering::Relaxed);
    println!("after add: {}", counter.load(Ordering::Relaxed)); // 52

    // fetch_and / fetch_or / fetch_xor — bitwise atomics
    counter.fetch_or(0b100, Ordering::Relaxed);
    println!("after or: {}", counter.load(Ordering::Relaxed)); // 52 | 4 = 56

    // swap — write a value, return the old one
    let old = counter.swap(0, Ordering::Relaxed);
    println!("swap: old={old}, new={}", counter.load(Ordering::Relaxed)); // old=56, new=0

    // compare_exchange — CAS (compare-and-swap): "if it's X, set it to Y"
    counter.store(10, Ordering::Relaxed);
    let result = counter.compare_exchange(10, 99, Ordering::Relaxed, Ordering::Relaxed);
    println!(
        "CAS success: {result:?}, value={}",
        counter.load(Ordering::Relaxed)
    ); // Ok(10), value=99

    let result = counter.compare_exchange(10, 200, Ordering::Relaxed, Ordering::Relaxed);
    println!(
        "CAS fail: {result:?}, value={}",
        counter.load(Ordering::Relaxed)
    ); // Err(99), value=99
}

// ===== Practical: AtomicBool flag (shared stop signal) =====

fn atomic_bool_flag() {
    println!("--- AtomicBool flag ---");
    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);

    let worker = thread::spawn(move || {
        while r.load(Ordering::Relaxed) {
            print!(".");
            thread::sleep(Duration::from_millis(100));
        }
        println!("\nworker saw stop signal, exiting");
    });

    thread::sleep(Duration::from_millis(500));
    running.store(false, Ordering::Relaxed); // signal from main thread
    worker.join().unwrap();
}

// ===== Practical: Atomic counter (no Mutex needed) =====

fn atomic_counter() {
    println!("--- Atomic counter (10 threads, 1000 increments each) ---");
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                c.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!(
        "final count: {} (expected 10000)",
        counter.load(Ordering::Relaxed)
    );
    // With a plain `usize` + `&mut`, threads can't share it.
    // With `Mutex<usize>`, it works but locks are heavy.
    // With `AtomicUsize`, zero locking — just CPU instructions.
}

// ===== AtomicPtr — atomically swap a pointer =====

fn atomic_ptr_demo() {
    println!("--- AtomicPtr ---");
    use std::sync::atomic::AtomicPtr;

    // NOTE: AtomicPtr only swaps the *pointer*, not the data
    static DATA: AtomicPtr<[i32; 3]> = AtomicPtr::new(std::ptr::null_mut());

    let arr = Box::into_raw(Box::new([1, 2, 3]));
    DATA.store(arr, Ordering::Release);

    // Another thread could atomically read it
    let ptr = DATA.load(Ordering::Acquire);
    if !ptr.is_null() {
        let val = unsafe { &*ptr };
        println!("ptr data: {val:?}");
    }
    // You need to free it manually when done
    unsafe { drop(Box::from_raw(ptr)) };
}

// ===== Ordering — what does Relaxed / Acquire / Release mean? =====

fn ordering_explained() {
    println!("--- Ordering levels ---");
    println!("Relaxed: no ordering guarantees, just the atomic op itself (fastest)");
    println!("Release: all writes BEFORE this store are visible to Acquire loads");
    println!("Acquire: all reads AFTER this load see Release-store writes");
    println!("AcqRel: Acquire + Release (for read-modify-write ops like fetch_add)");
    println!("SeqCst: strongest — single total order across ALL threads (slowest)");

    // 99% of the time you use one of:
    //   Ordering::Relaxed  — for counters, flags (no one depends on seeing other data)
    //   Ordering::SeqCst   — when you're unsure (correct, just slow)
    //   Ordering::Release  — paired with Acquire for safe pointer/data sharing
}

// ===== Mutex vs Atomic: when to use what =====

fn mutex_vs_atomic() {
    println!("--- Mutex vs Atomic ---");

    // ATOMIC: single integer/bool/pointer, simple operations
    let counter = AtomicUsize::new(0);
    counter.fetch_add(1, Ordering::Relaxed);

    // MUTEX: complex data, multiple fields, anything > 8 bytes
    let data = std::sync::Mutex::new(vec![1, 2, 3]);
    data.lock().unwrap().push(4);

    println!("Atomic: one number (counter, flag, ID allocator)");
    println!("Mutex: any complex data (structs, Vec, HashMap)");
}

fn main() {
    atomic_methods();
    atomic_bool_flag();
    atomic_counter();
    // atomic_ptr_demo();  // advanced — uncomment to try
    ordering_explained();
    mutex_vs_atomic();
}
