//! Iterator optimization guide — zero-cost, fusion, and itertools.
//! Run: cargo run --example iterator-opt

// NOTE: all examples are annotated with what the compiler generates.

// =========================================================================
// 1. Zero-cost abstraction: what it actually means
// =========================================================================
//
// "You don't pay for what you don't use. You couldn't write it better by hand."
//   — Stroustrup (C++), applies equally to Rust's iterators
//
// Iterator chains compile down to the SAME machine code as hand-written loops.
// The chain is fully inlined and optimized away at compile time.

fn zero_cost_demo() {
    let data: Vec<i64> = (0..10_000_000).collect();

    // Hand-rolled loop:
    let mut sum = 0i64;
    for &x in &data {
        if x % 2 == 0 {
            sum += x;
        }
    }

    // Iterator chain (compiles to the same thing):
    let sum2: i64 = data.iter().filter(|&&x| x % 2 == 0).sum();

    assert_eq!(sum, sum2);
    // ↓
    // Both produce identical x86-64 assembly when compiled with optimizations.
    // The filter closure is inlined. sum() unrolls the loop. No overhead.
}

// =========================================================================
// 2. Fusion — chaining doesn't allocate intermediate collections
// =========================================================================
//
// Each adapter (filter, map, take, etc.) is a struct wrapping the previous.
// No Vec allocations happen until .collect().

fn fusion_demo() {
    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // This:
    let result: Vec<_> = data
        .iter()
        .filter(|&&x| x > 3) // Filter struct
        .map(|&x| x * 2) // Map struct wrapping Filter
        .take(3) // Take struct wrapping Map wrapping Filter
        .collect(); // ONE pass, ONE allocation

    // Compiles to the SAME assembly as:
    let mut result2 = Vec::new();
    let mut count = 0;
    for &x in &data {
        if x > 3 && count < 3 {
            result2.push(x * 2);
            count += 1;
        }
    }

    assert_eq!(result, result2);
    // No intermediate arrays. No overhead.
}

// =========================================================================
// 3. Early termination — iterators STOP when you don't need more
// =========================================================================
//
// .any(), .all(), .find(), .position(), .take() all SHORT-CIRCUIT.

fn early_termination() {
    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // ❌ Bad: allocates then checks
    let _: Vec<_> = data.iter().filter(|&&x| x > 5).collect();
    // ... do something with first element only — wasted 4 allocations

    // ✅ Good: .find() stops after first match
    let first_big = data.iter().find(|&&x| x > 5);
    println!("First > 5: {first_big:?}");

    // .any() / .all() — stops at first mismatch
    let all_positive = data.iter().all(|&x| x > 0); // stops after 1st element (it returns true immediately)
    let any_negative = data.iter().any(|&x| x < 0); // checks all 10 (none found)

    // .take() — limited iteration
    let first_three: Vec<_> = data.iter().take(3).collect();
    println!("First 3: {first_three:?}");

    // .skip() — skip first N
    let after_5: Vec<_> = data.iter().skip(5).take(3).collect();
    println!("Skip 5, take 3: {after_5:?}");
}

// =========================================================================
// 4. Size hints — iterators tell Vec how much to pre-allocate
// =========================================================================
//
// Most iterators implement ExactSizeIterator. .collect() uses size_hint()
// to call Vec::with_capacity() instead of repeated push() reallocs.

fn size_hint_demo() {
    let data: Vec<i64> = (0..1_000_000).collect();

    // .filter() can't know the size in advance (no exact hint)
    // But .map() preserves the exact hint from the source
    // So this pre-allocates correctly:
    let doubled: Vec<_> = data.iter().map(|&x| x * 2).collect();
    // -> Vec::with_capacity(data.len()), then fill
    // No reallocations.

    // .filter() + .collect() may reallocate a few times (like push()),
    // but it's still amortized O(1) per element.
}

// =========================================================================
// 5. Flattening — avoid collecting intermediate Vecs
// =========================================================================

fn flatten_perf() {
    let users = vec![vec!["a", "b"], vec!["c", "d", "e"], vec!["f"]];

    // ❌ Bad: allocates an intermediate Vec<&&str>
    let flat: Vec<&&str> = users.iter().flat_map(|v| v.iter()).collect();

    // ✅ Good (same thing, no extra allocation possible here)
    // flat_map doesn't collect the inner iterators — it chains them.
    // The inner iterators yield elements one by one into the outer stream.
    // This is just as efficient as the hand-written double loop.

    // Even more efficient: avoid collecting at all if you're going to iterate
    for item in users.iter().flat_map(|v| v.iter()) {
        // process one item at a time, zero allocations
        let _ = item;
    }
}

// =========================================================================
// 6. Chain vs collecting + extending
// =========================================================================

fn chain_vs_extend() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];

    // ❌ Bad: allocates intermediate Vec
    let mut result = a.clone();
    result.extend(&b); // may reallocate

    // ✅ Good: chain lazy, collect once
    let result: Vec<_> = a.iter().chain(b.iter()).copied().collect();
    // .chain() doesn't copy anything until .collect()
}

// =========================================================================
// 7. The one place iterators CAN be slower: tight cache-hot loops
// =========================================================================
//
// In extremely hot loops (hundreds of millions of iterations) where every
// nanosecond matters, a raw loop can be ~5-10% faster because:
//   - The iterator state machine has slightly more register pressure
//   - Loop unrolling heuristics differ slightly
//
// For 99.9% of code: iterators are identical or faster (optimizer sees
// through the abstraction better).

fn hot_loop_comparison() {
    let data: Vec<i64> = (0..100_000).collect();

    // These compile to the same thing in practice:
    let mut sum1 = 0i64;
    for &x in &data {
        sum1 += x;
    }

    let sum2: i64 = data.iter().sum();

    assert_eq!(sum1, sum2);
    // Only benchmark to decide. Usually identical.
}

// =========================================================================
// 8. itertools — the good stuff
// =========================================================================
//
// Add: cargo add itertools
//
// Every method below replaces a pattern that would require a manual loop
// or multiple collect() calls.

use std::collections::HashMap;

fn itertools_patterns() {
    // (These examples show what itertools::Itertools provides.
    //  Uncomment the `use` line and add itertools to your Cargo.toml to run them.)

    // --- .unique() — dedup without sorting or HashSet ---
    // let nums = vec![1, 2, 2, 3, 3, 3, 4];
    // let unique: Vec<_> = nums.into_iter().unique().collect();
    // // [1, 2, 3, 4]  — uses a HashSet internally but you don't write it
    //
    // --- .sorted() — collect+sort in one call ---
    // let words = vec!["banana", "apple", "cherry"];
    // let sorted: Vec<_> = words.into_iter().sorted().collect();
    // // ["apple", "banana", "cherry"]
    //
    // --- .counts() — frequency map in one call ---
    // let items = vec!["a", "b", "a", "c", "a", "b"];
    // let freq: HashMap<&str, usize> = items.into_iter().counts();
    // // {"a": 3, "b": 2, "c": 1}
    //
    // --- .cartesian_product() — nested loops for independent sets ---
    // for (x, y) in (0..3).cartesian_product(0..3) {
    //     // (0,0), (0,1), (0,2), (1,0), (1,1), ...
    // }
    //
    // --- .tuple_windows() — sliding window ---
    // let nums = vec![1, 2, 3, 4, 5];
    // for (a, b, c) in nums.iter().tuple_windows() {
    //     // (1,2,3), (2,3,4), (3,4,5)
    // }
    // Replaces: for i in 0..nums.len()-2 { (nums[i], nums[i+1], nums[i+2]) }
    //
    // --- .positions() — find all indices matching a predicate ---
    // let nums = vec![1, 2, 3, 4, 5, 6];
    // let evens: Vec<usize> = nums.iter().positions(|&x| x % 2 == 0).collect();
    // // [1, 3, 5]
    //
    // --- .interleave() — zip but one runs out first ---
    // let a = vec![1, 3, 5];
    // let b = vec![2, 4, 6, 7, 8];
    // let merged: Vec<_> = a.into_iter().interleave(b).collect();
    // // [1, 2, 3, 4, 5, 6, 7, 8]
    //
    // --- .intersperse() — insert between elements ---
    // let words = ["a", "b", "c"];
    // let csv: String = words.iter().copied().intersperse(",").collect();
    // // "a,b,c"
    // // Replaces: manual loop with "is_first" flag
    //
    // --- .partition() — split into two collections ---
    // let nums = vec![1, 2, 3, 4, 5, 6];
    // let (evens, odds): (Vec<_>, Vec<_>) = nums.into_iter().partition(|x| x % 2 == 0);
    // // evens: [2, 4, 6], odds: [1, 3, 5]
    //
    // --- .group_by() — group consecutive equal elements ---
    // let nums = vec![1, 1, 2, 2, 2, 3, 1, 1];
    // for (key, group) in &nums.into_iter().group_by(|x| *x) {
    //     println!("{key}: {:?}", group.collect::<Vec<_>>());
    //     // 1: [1, 1], 2: [2, 2, 2], 3: [3], 1: [1, 1]
    // }
    //
    // --- .chunks() — batch processing ---
    // let nums = vec![1, 2, 3, 4, 5, 6, 7];
    // for chunk in &nums.into_iter().chunks(3) {
    //     // [1,2,3], [4,5,6], [7]
    // }
    //
    // --- .all_equal() — check all elements same ---
    // let a = vec![1, 1, 1];
    // let b = vec![1, 2, 1];
    // println!("{} {}", a.iter().all_equal(), b.iter().all_equal());
    // // true, false
    //
    // --- .merge() — merge two sorted iterators ---
    // let a = vec![1, 3, 5];
    // let b = vec![2, 4, 6];
    // let merged: Vec<_> = a.into_iter().merge(b).collect();
    // // [1, 2, 3, 4, 5, 6]
    //
    // --- .kmerge() — merge N sorted iterators ---
    // let a = vec![1, 4];
    // let b = vec![2, 5];
    // let c = vec![3, 6];
    // let merged: Vec<_> = vec![a, b, c].into_iter().kmerge().collect();
    // // [1, 2, 3, 4, 5, 6]
}

// =========================================================================
// 9. Real-world: before and after
// =========================================================================

type User = (i32, String, bool, Vec<String>);

fn users() -> Vec<User> {
    vec![
        (1, "Alice".into(), true, vec!["rust".into(), "db".into()]),
        (2, "Bob".into(), false, vec![]),
        (3, "Charlie".into(), true, vec![
            "frontend".into(),
            "rust".into(),
            "ui".into(),
        ]),
        (4, "Diana".into(), true, vec!["db".into(), "ops".into()]),
    ]
}

fn real_world_before() {
    // Goal: unique sorted tags from active users, as a comma-separated string
    let users = users();

    // ❌ Nested mess
    let mut active_users = Vec::new();
    for user in &users {
        if user.2 {
            // active
            active_users.push(user);
        }
    }

    let mut all_tags = Vec::new();
    for user in &active_users {
        for tag in &user.3 {
            // tags
            if !all_tags.contains(tag) {
                all_tags.push(tag.clone());
            }
        }
    }

    all_tags.sort();

    let mut result = String::new();
    for (i, tag) in all_tags.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(tag);
    }

    println!("Before: {result}");
}

fn real_world_after() {
    let users = users();

    // ✅ Iterator chain, zero nesting
    let result = users
        .iter()
        .filter(|u| u.2) // active only
        .flat_map(|u| &u.3) // collect all tags
        .map(|s| s.as_str())
        .collect::<std::collections::BTreeSet<_>>() // unique + sorted
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    println!("After:  {result}");
}

fn real_world_itertools() {
    // With itertools — even simpler:
    // use itertools::Itertools;
    //
    // let result: String = users
    //     .iter()
    //     .filter(|u| u.2)
    //     .flat_map(|u| &u.3)
    //     .map(|s| s.as_str())
    //     .unique()
    //     .sorted()
    //     .intersperse(", ")
    //     .collect();
}

fn main() {
    zero_cost_demo();
    fusion_demo();
    early_termination();
    // flatten_perf();
    // chain_vs_extend();
    real_world_before();
    real_world_after();

    // Reminder: itertools needs to be added to Cargo.toml
    println!("\nAdd itertools: cargo add itertools");
    println!("Then uncomment the itertools examples and add: use itertools::Itertools;");
}
