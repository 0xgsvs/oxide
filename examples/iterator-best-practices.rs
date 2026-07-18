//! Iterator best practices: allocation avoidance, pre-allocation, O(n²) avoidance.
//! Run: cargo run --example iterator-best-practices

use std::collections::HashMap;

// =========================================================================
// BEST PRACTICE 1: Don't collect if you're just going to iterate again
// =========================================================================
//
// ❌ Common mistake: collect into Vec, then iterate
//
//   let users: Vec<_> = db.fetch_all().await.iter().filter(...).collect();
//   for user in &users { ... }        ← second pass over same data
//
// ✅ Pass the iterator directly, collect only at the final consumer

fn dont_collect_needlessly() {
    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // ❌ Bad: collect intermediate, then iterate
    let evens: Vec<_> = data.iter().filter(|&&x| x % 2 == 0).collect();
    for &x in &evens {
        println!("  even: {x}");
    }

    // ✅ Good: iterate the filter directly, no allocation
    for &x in data.iter().filter(|&&x| x % 2 == 0) {
        println!("  even: {x}");
    }

    // If you need both filtered and unfiltered — use partition
    let (evens, odds): (Vec<_>, Vec<_>) =
        data.iter().copied().partition::<Vec<_>, _>(|&x| x % 2 == 0);
    println!("  evens: {evens:?}, odds: {odds:?}");
}

// =========================================================================
// BEST PRACTICE 2: Pass iterators, not collections
// =========================================================================

// ❌ Bad: takes a Vec (forces the caller to allocate)
fn process_slice_bad(items: &[i32]) -> i64 {
    items.iter().map(|&x| x as i64).sum()
}

// ✅ Good: takes an iterator (caller can pass Vec, slice, Range, chain, etc.)
fn process_iter_good(items: impl Iterator<Item = i32>) -> i64 {
    items.map(|x| x as i64).sum()
}

fn pass_iterator_not_collection() {
    let vec = vec![1, 2, 3];
    let arr = [4, 5, 6];
    let range = 7..=9;

    // All three work without forcing the caller to collect into Vec
    let sum = process_iter_good(vec.into_iter())
        + process_iter_good(arr.into_iter())
        + process_iter_good(range.into_iter());

    println!("  sum from iterators: {sum}");

    // Even chains work:
    let chained = process_iter_good(
        vec![1, 2, 3]
            .into_iter()
            .chain([4, 5, 6])
            .chain(7..=9)
            .map(|x| x * 2),
    );
    println!("  chained: {chained}");

    // Static dispatch (impl Iterator) is zero-cost.
    // The compiler monomorphizes a separate function for each iterator type.
    // No vtables, no boxing, no allocation.
}

// =========================================================================
// BEST PRACTICE 3: Pre-allocate when you know the size
// =========================================================================
//
// .collect() already does this when the source has an exact size hint.
// But when you're building manually, use with_capacity.

fn pre_allocate() {
    let data = [10, 20, 30, 40, 50];

    // ❌ Bad: Vec grows dynamically (may reallocate 2-3 times)
    let mut result = Vec::new();
    for &x in &data {
        if x > 20 {
            result.push(x * 2);
        }
    }

    // ✅ Good: pre-allocate (though filter makes exact size unknown,
    //    so this is actually a case where you CAN'T pre-allocate perfectly)
    let mut result = Vec::with_capacity(data.len()); // upper bound
    for &x in &data {
        if x > 20 {
            result.push(x * 2);
        }
    }

    // ✅ Best: let collect() handle it (same result, less code)
    let result: Vec<_> = data.iter().filter(|&&x| x > 20).map(|&x| x * 2).collect();

    // When you KNOW the exact size — use with_capacity:
    let n = 1000;
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        ids.push(i);
    }
    // Zero reallocations.

    // Iterator equivalent (pre-allocates via ExactSizeIterator):
    let ids: Vec<_> = (0..n).collect();
    // Same thing.
}

// =========================================================================
// BEST PRACTICE 4: Avoid O(n²) — use HashMap or sort+dedup
// =========================================================================

fn avoid_n_squared() {
    let users = vec![
        (1, "Alice"),
        (2, "Bob"),
        (3, "Charlie"),
        (4, "Alice"), // duplicate name
        (5, "Charlie"),
    ];

    // ===== Example A: Find unique names =====

    // ❌ O(n²) — linear search in a Vec for every element
    let mut unique = Vec::new();
    for &(_, name) in &users {
        if !unique.contains(&name) {
            // O(n) scan
            unique.push(name);
        }
    }
    println!("  O(n²) unique: {unique:?}"); // ~25 comparisons for 5 items

    // ✅ O(n) — use a HashSet
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for &(_, name) in &users {
        if seen.insert(name) {
            // O(1) hash lookup
            unique.push(name);
        }
    }
    println!("  O(n) unique: {unique:?}");

    // ✅ Even better — use itertools .unique() (wraps HashSet internally)
    // use itertools::Itertools;
    // let unique: Vec<_> = users.iter().map(|(_, n)| *n).unique().collect();

    // ===== Example B: Group users by name =====

    // ❌ O(n²) — nested loop
    let names: Vec<&str> = users.iter().map(|(_, n)| *n).collect();
    let mut by_name: Vec<(&str, Vec<i32>)> = Vec::new();
    for &name in &names {
        if !by_name.iter().any(|(n, _)| n == &name) {
            // O(n) scan
            let ids: Vec<i32> = users
                .iter()
                .filter(|(_, n)| n == &name)
                .map(|(id, _)| *id)
                .collect(); // another O(n)
            by_name.push((name, ids));
        }
    }
    // Total: O(n²) — three nested loops

    // ✅ O(n) — single pass with HashMap
    let mut by_name: HashMap<&str, Vec<i32>> = HashMap::new();
    for &(id, name) in &users {
        by_name.entry(name).or_default().push(id);
    }
    println!("  O(n) grouped: {by_name:?}");

    // ===== Example C: Find pairs that sum to target =====

    let nums = [2, 7, 11, 15, 3, 6, 8];

    // ❌ O(n²) — nested loop
    let target = 9;
    let mut pairs = Vec::new();
    for i in 0..nums.len() {
        for j in i + 1..nums.len() {
            if nums[i] + nums[j] == target {
                pairs.push((nums[i], nums[j]));
            }
        }
    }
    println!("  O(n²) pairs summing to {target}: {pairs:?}");

    // ✅ O(n) — use a hash set
    let mut seen = std::collections::HashSet::new();
    let mut pairs = Vec::new();
    for &x in &nums {
        let complement = target - x;
        if seen.contains(&complement) {
            pairs.push((complement, x));
        }
        seen.insert(x);
    }
    println!("  O(n) pairs summing to {target}: {pairs:?}");
}

// =========================================================================
// BEST PRACTICE 5: Avoid cloning — use references through the chain
// =========================================================================

fn avoid_cloning() {
    let data = vec!["hello".to_string(), "world".to_string()];

    // ❌ Bad: clones every string
    let uppercased: Vec<String> = data.iter().map(|s| s.to_uppercase()).collect();

    // ✅ Good: if the consumer just reads, pass &str references
    for s in data.iter().map(|s| s.as_str()) {
        let _ = s; // no allocation
    }

    // If you need owned values, clone at the last moment, not earlier:
    let uppercased: Vec<String> = data
        .iter()
        .filter(|s| s.len() > 3)
        .map(|s| s.to_uppercase()) // ONE allocation per element, at the end
        .collect();

    // .copied() — cheap: copies Copy types (no allocation)
    let nums = vec![1, 2, 3];
    let doubled: Vec<i32> = nums.iter().copied().map(|x| x * 2).collect();
    // .copied() is just a bitwise copy of i32 — same as if you dereferenced

    // .cloned() — cheap for Copy types, expensive for String/Vec
    // Prefer .copied() when the type is Copy (it's more explicit)
}

// =========================================================================
// BEST PRACTICE 6: Use BTreeMap/BTreeSet for sorted+unique in one pass
// =========================================================================
//
// Instead of: collect → sort → dedup
// Just: collect into BTreeSet (O(n log n), sorted + unique)

fn btreeset_for_sorted_unique() {
    let data = [5, 3, 1, 3, 4, 2, 5, 1];

    // ❌ Multiple passes:
    let mut v: Vec<_> = data.to_vec();
    v.sort(); // O(n log n)
    v.dedup(); // O(n)
    // Total: O(n log n) + O(n) + 2 allocations (vec + sort buf)

    // ✅ One pass:
    let unique_sorted: std::collections::BTreeSet<_> = data.iter().copied().collect();
    // Total: O(n log n), single allocation, naturally sorted
    println!("  BTreeSet: {unique_sorted:?}"); // {1, 2, 3, 4, 5}

    // Convert back to Vec if needed:
    let v: Vec<_> = unique_sorted.into_iter().collect();
}

// =========================================================================
// BEST PRACTICE 7: Lazy evaluation — don't compute what you don't need
// =========================================================================

fn lazy_evaluation() {
    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // ❌ Collects all results, then takes first 3
    let first3: Vec<_> = data
        .iter()
        .map(|x| {
            println!("  computing expensive transform for {x}");
            x * 2
        })
        .collect::<Vec<_>>() // ALL 10 computed!
        .into_iter()
        .take(3)
        .collect();

    // ✅ Only computes the first 3
    let first3: Vec<_> = data
        .iter()
        .map(|x| {
            println!("  computing expensive transform for {x}");
            x * 2
        })
        .take(3) // stops after 3 — remaining 7 never computed!
        .collect();

    println!("  First 3: {first3:?}");
    // Only "computing expensive transform for 1, 2, 3" is printed
}

// =========================================================================
// BEST PRACTICE 8: Use filter_map to combine filter + map
// =========================================================================

fn filter_map_pattern() {
    let inputs = ["42", "abc", "55", "xyz", "100"];

    // ❌ Two passes: filter then map
    let nums: Vec<i32> = inputs
        .iter()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();
    println!("  filter_map: {nums:?}");

    // This is equivalent to filter + map but does it in ONE pass:
    let nums: Vec<i32> = inputs
        .iter()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();
}

// =========================================================================
// BEST PRACTICE 9: Use .find_map() to combine find + map
// =========================================================================

fn find_map_pattern() {
    let users = [(1, "Alice", true), (2, "Bob", false), (3, "Charlie", true)];

    // ❌ Find then map
    let first_active_id = users
        .iter()
        .find(|&&(_, _, active)| active)
        .map(|&(id, _, _)| id);

    // ✅ find_map — one pass, no Option nesting
    let first_active_id = users
        .iter()
        .find_map(|&(id, _, active)| if active { Some(id) } else { None });

    // Even cleaner with filter_map + next:
    let first_active_id = users
        .iter()
        .filter_map(|&(id, _, active)| if active { Some(id) } else { None })
        .next();

    println!("  First active user id: {first_active_id:?}");
}

// =========================================================================
// BEST PRACTICE 10: Use .flatten() for nested Options
// =========================================================================

fn flatten_options() {
    let data = vec![Some(Some(1)), Some(Some(2)), Some(None), None];

    // ❌ Nested unwraps and filters
    let result: Vec<i32> = data
        .iter()
        .filter_map(|x| x.as_ref())
        .filter_map(|x| x.as_ref())
        .copied()
        .collect();

    // ✅ flatten — one call per nesting level
    let result: Vec<i32> = data
        .iter()
        .flatten() // removes outer None, yields &Some(i32) or &None
        .flatten() // removes inner None, yields &i32
        .copied() // &i32 → i32
        .collect();

    println!("  Flattened options: {result:?}");
}

fn main() {
    println!("=== 1. Don't collect if you'll iterate again ===");
    dont_collect_needlessly();

    println!("\n=== 2. Pass iterators, not collections ===");
    pass_iterator_not_collection();

    println!("\n=== 3. Pre-allocate ===");
    pre_allocate();

    println!("\n=== 4. Avoid O(n²) ===");
    avoid_n_squared();

    println!("\n=== 5. Avoid cloning ===");
    avoid_cloning();

    println!("\n=== 6. BTreeSet for sorted+unique ===");
    btreeset_for_sorted_unique();

    println!("\n=== 7. Lazy evaluation ===");
    lazy_evaluation();

    println!("\n=== Summary ===");
    println!("  ❌ .collect() if you'll iterate right after");
    println!("  ✅ Pass impl Iterator, not &[T]");
    println!("  ❌ .contains() in a Vec loop → O(n²)");
    println!("  ✅ .collect::<HashSet<_>>() or .unique() → O(n)");
    println!("  ❌ Vec::new() + push → may reallocate");
    println!("  ✅ Vec::with_capacity(n) → zero reallocs");
    println!("  ❌ collect → sort → dedup → 3 passes");
    println!("  ✅ BTreeSet → 1 pass, sorted + unique");
    println!("  ❌ .clone() in a chain → allocate early");
    println!("  ✅ .copied() for Copy types → zero cost");
    println!("  ❌ .take(n) after .collect() → compute everything");
    println!("  ✅ .take(n) before .collect() → lazy, compute only n");
    println!("  ❌ .filter().map() on same condition → 2 passes");
    println!("  ✅ .filter_map() → 1 pass");
    println!("  ❌ Nested Option→ filter_map inside filter_map");
    println!("  ✅ .flatten().flatten() → clean");
}
