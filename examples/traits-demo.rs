//! Run: cargo run --example traits-demo

// ===== 1. Copy vs Drop — mutually exclusive =====
#[derive(Debug, Copy, Clone)]
struct Point {
    x: i32,
    y: i32,
}
// ✅ Copy + Clone — stack-only, drop is a no-op

#[derive(Debug, Clone)]
struct NamedPoint {
    x: i32,
    y: i32,
    name: String,
}
// ❌ can't be Copy — String owns heap memory, Drop must free it
// ✅ Clone only — need explicit .clone()

fn copy_vs_clone() {
    // Copy = silent memcpy at every assignment
    let a = Point { x: 1, y: 2 };
    let b = a; // invisible bitwise copy
    println!("a: {a:?} b: {b:?}"); // both alive

    // Clone = explicit method call
    let c = NamedPoint {
        x: 1,
        y: 2,
        name: "origin".into(),
    };
    let d = c.clone(); // explicit: you SEE the heap alloc
    println!("c: {c:?} d: {d:?}");
}

// ===== 2. Hasher trait (Hash + Hasher) =====
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn hash_example() {
    let s = "hello";
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher); // feeds bytes to the hasher
    let hash = hasher.finish(); // produces u64
    println!("hash of 'hello': {:x}", hash);

    // Custom types: #[derive(Hash)] feeds all fields in order
    #[derive(Hash)]
    struct Person {
        name: String,
        age: u8,
    }

    let p = Person {
        name: "Alice".into(),
        age: 30,
    };
    let mut h = DefaultHasher::new();
    p.hash(&mut h);
    println!("hash of Alice: {:x}", h.finish());

    // Hasher is the *engine* — you swap it for different algorithms
    // DefaultHasher = SipHash-2-4 (DoS-resistant)
    // You use it indirectly via #[derive(Hash)] + collections
}

fn hashmap_key_example() {
    // The real use: types as HashMap keys
    #[derive(Hash, PartialEq, Eq, Debug)]
    struct Key {
        id: u32,
        name: String,
    }

    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(
        Key {
            id: 1,
            name: "foo".into(),
        },
        "value1",
    );
    map.insert(
        Key {
            id: 2,
            name: "bar".into(),
        },
        "value2",
    );
    // HashMap calls Hash + Eq internally
    println!(
        "{:?}",
        map.get(&Key {
            id: 1,
            name: "foo".into()
        })
    );
}

// ===== 3. FnOnce / FnMut / Fn — examples =====

/// Takes a closure it can call exactly once (consumes it)
fn run_once<F: FnOnce()>(f: F) {
    f();
    // f(); ❌ can't call twice — FnOnce consumes the closure
}

/// Takes a closure that can mutate its captures
fn run_mut<F: FnMut(i32) -> bool>(mut f: F) -> Vec<i32> {
    (0..5).filter(|x| f(*x)).collect()
}

/// Takes a closure that only reads captures — callable many times, safely
fn run_fn<F: Fn(i32) -> i32>(f: F, values: &[i32]) -> Vec<i32> {
    values.iter().map(|&x| f(x)).collect()
}

fn closure_examples() {
    // === FnOnce ===
    let s = String::from("I am consumed");
    let consume = || drop(s); // takes ownership of s
    run_once(consume);
    // drop(consume);  ❌ already consumed by run_once
    // println!("{s}"); ❌ s was dropped inside the closure

    // === FnMut ===
    let mut counter = 0;
    let count_and_check = |x: i32| -> bool {
        counter += 1; // &mut capture
        x > 2
    };
    let filtered = run_mut(count_and_check);
    println!("filtered: {filtered:?}, counter was called");
    // counter is now 5 (one call per element)

    // === Fn (pure read) ===
    let multiplier = 3;
    let triple = |x: i32| x * multiplier; // only reads multiplier
    let result = run_fn(triple, &[1, 2, 3, 4, 5]);
    println!("tripled: {result:?}");

    // === Practical: sorting with custom key ===
    let mut words = vec!["banana", "apple", "cherry", "date"];
    // .sort_by() takes FnMut — can capture state
    let mut count = 0u32;
    words.sort_by(|a, b| {
        count += 1;
        a.len().cmp(&b.len())
    });
    println!("sorted by length: {words:?} (compared {count} times)");
}

// ===== 4. PartialEq / Eq / PartialOrd / Ord — the NaN case =====

fn partial_vs_full() {
    // PartialEq only — NaN breaks reflexivity
    let nan = f64::NAN;
    println!("NaN == NaN: {}", nan == nan); // false
    println!("NaN.partial_cmp(&3.0): {:?}", nan.partial_cmp(&3.0)); // None

    // Eq — every value == itself
    // All integer types, strings, bools, etc.
    let x = 42i32;
    println!("42 == 42: {}", x == x); // true — always
    // That's why i32 is Eq, f64 is only PartialEq

    // PartialOrd — can return None
    // Ord — always returns Less/Equal/Greater
    // .sort() requires Ord — that's why Vec<f64>.sort() doesn't compile
    let mut nums = vec![3.0, 1.0, f64::NAN, 2.0];
    // nums.sort();  // ❌ f64: !Ord
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));
    println!("sorted floats: {nums:?}");
}

// ===== 5. Send + Sync — compiler checks for you =====
use std::{
    sync::{Arc, Mutex},
    thread,
};

fn send_sync_demo() {
    let data = Arc::new(Mutex::new(vec![1, 2, 3]));

    let mut handles = vec![];
    for i in 0..3 {
        let d = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            let mut guard = d.lock().unwrap();
            guard.push(i);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!("shared data: {:?}", data.lock().unwrap());
    // Arc<T>: Send + Sync when T: Send + Sync
    // Mutex<T>: Send + Sync when T: Send
    // The compiler auto-derives these. No manual impl needed.
}

fn main() {
    println!("=== Copy vs Clone ===");
    copy_vs_clone();

    println!("\n=== Hash + Hasher ===");
    hash_example();
    hashmap_key_example();

    println!("\n=== FnOnce / FnMut / Fn ===");
    closure_examples();

    println!("\n=== PartialEq / PartialOrd vs Eq / Ord ===");
    partial_vs_full();

    println!("\n=== Send + Sync ===");
    send_sync_demo();
}
