//! Flatten nested loops/ifs using iterators + itertools.
//! Run: cargo run --example flatten-demo

use std::collections::HashMap;

// =========================================================================
// Core idea: replace "what" (loops + ifs) with "what you want" (iterators)
// =========================================================================

#[derive(Debug)]
struct User {
    id: i32,
    name: String,
    role: String,
    active: bool,
    tags: Vec<String>,
}

fn users() -> Vec<User> {
    vec![
        User {
            id: 1,
            name: "Alice".into(),
            role: "admin".into(),
            active: true,
            tags: vec!["rust".into(), "db".into()],
        },
        User {
            id: 2,
            name: "Bob".into(),
            role: "member".into(),
            active: false,
            tags: vec![],
        },
        User {
            id: 3,
            name: "Charlie".into(),
            role: "member".into(),
            active: true,
            tags: vec!["frontend".into(), "rust".into(), "ui".into()],
        },
        User {
            id: 4,
            name: "Diana".into(),
            role: "admin".into(),
            active: true,
            tags: vec!["db".into(), "ops".into()],
        },
    ]
}

fn main() {
    let users = users();

    // =====================================================================
    // PATTERN 1: filter + collect — replaces "for + if"
    // =====================================================================
    println!("=== 1. Replace for + if with filter ===");

    // ❌ Nested loop + if
    let mut active = vec![];
    for user in &users {
        if user.active && user.role == "admin" {
            active.push(user.name.clone());
        }
    }
    println!("  Old: {active:?}");

    // ✅ Iterator chain (same logic, zero nesting)
    let active: Vec<_> = users
        .iter()
        .filter(|u| u.active && u.role == "admin")
        .map(|u| &u.name)
        .collect();
    println!("  New: {active:?}");

    // =====================================================================
    // PATTERN 2: flatten — replaces nested loops over nested collections
    // =====================================================================
    println!("\n=== 2. Replace nested loops with flatten ===");

    // ❌ Nested loop: for each user, for each tag
    let mut tag_pairs = vec![];
    for user in &users {
        for tag in &user.tags {
            tag_pairs.push((&user.name, tag));
        }
    }
    println!("  Old: {tag_pairs:?}");

    // ✅ Flat iterator chain
    let tag_pairs: Vec<_> = users
        .iter()
        .flat_map(|u| u.tags.iter().map(move |t| (&u.name, t)))
        .collect();
    println!("  New: {tag_pairs:?}");

    // Even simpler: just collect all tags
    let all_tags: Vec<&str> = users
        .iter()
        .flat_map(|u| &u.tags)
        .map(|s| s.as_str())
        .collect();
    println!("  All tags: {all_tags:?}");

    // =====================================================================
    // PATTERN 3: Option combinators — replaces nested if-let
    // =====================================================================
    println!("\n=== 3. Replace nested if-let with Option combinators ===");

    fn parse_and_double(s: &str) -> Option<i32> {
        // ❌ Nested if-let
        if let Ok(n) = s.parse::<i32>() {
            if n > 0 {
                return Some(n * 2);
            }
        }
        None
    }

    fn parse_and_double_flat(s: &str) -> Option<i32> {
        // ✅ Chain: flat_map + filter + map
        s.parse::<i32>().ok().filter(|&n| n > 0).map(|n| n * 2)
    }

    for val in ["42", "-5", "abc"] {
        println!(
            "  {val}: {}/{}",
            parse_and_double(val).is_some(),
            parse_and_double_flat(val).is_some()
        );
    }

    // =====================================================================
    // PATTERN 4: and_then / or_else — replaces nested if-let chains
    // =====================================================================
    println!("\n=== 4. Nested Option chains with and_then ===");

    fn find_admin_tag<'a>(users: &'a [User], name: &str) -> Option<&'a str> {
        // ❌ Nested if-let galore
        for user in users {
            if user.name == name && user.active {
                for tag in &user.tags {
                    if tag == "rust" {
                        return Some(tag);
                    }
                }
            }
        }
        None
    }

    fn find_admin_tag_flat<'a>(users: &'a [User], name: &str) -> Option<&'a str> {
        // ✅ Pure iterator chain, zero nesting
        users
            .iter()
            .find(|u| u.name == name && u.active)
            .and_then(|u| u.tags.iter().find(|t| *t == "rust"))
            .map(|s| s.as_str())
    }

    println!(
        "  Alice's rust tag: {:?}",
        find_admin_tag_flat(&users, "Alice")
    );
    println!("  Bob's rust tag: {:?}", find_admin_tag_flat(&users, "Bob"));

    // =====================================================================
    // PATTERN 5: fold / reduce — replaces for + accumulator
    // =====================================================================
    println!("\n=== 5. Replace for + accumulator with fold ===");

    // ❌ Manual loop + accumulator
    let mut total = 0;
    for user in &users {
        total += user.id;
    }
    println!("  Old sum: {total}");

    // ✅ fold or sum
    let total: i32 = users.iter().map(|u| u.id).sum();
    println!("  New sum: {total}");

    // Total tag count
    let total_tags: usize = users.iter().map(|u| u.tags.len()).sum();
    println!("  Total tags: {total_tags}");

    // Custom fold: count chars in all names
    let chars: usize = users.iter().fold(0, |acc, u| acc + u.name.len());
    println!("  Name chars: {chars}");

    // =====================================================================
    // PATTERN 6: group_by / chunk — replaces group-by loops
    // =====================================================================
    println!("\n=== 6. Group by role ===");

    // ❌ Manual grouping
    let mut by_role: HashMap<&str, Vec<&User>> = HashMap::new();
    for user in &users {
        by_role.entry(&user.role).or_default().push(user);
    }
    for (role, members) in &by_role {
        println!("  Old {role}: {} members", members.len());
    }

    // ✅ Iterator + collect (does the same, one expression)
    let by_role: HashMap<&str, Vec<&User>> = users.iter().fold(HashMap::new(), |mut acc, u| {
        acc.entry(u.role.as_str()).or_insert_with(Vec::new).push(u);
        acc
    });
    for (role, members) in &by_role {
        println!("  New {role}: {} members", members.len());
    }

    // =====================================================================
    // PATTERN 7: all / any — replaces boolean flag loops
    // =====================================================================
    println!("\n=== 7. Replace flag variables with all/any ===");

    // ❌ Loop with a boolean flag
    let mut all_active = true;
    for user in &users {
        if !user.active {
            all_active = false;
            break;
        }
    }
    println!("  Old all active: {all_active}");

    // ✅ Iterator
    let all_active = users.iter().all(|u| u.active);
    println!("  New all active: {all_active}");

    let any_admin = users.iter().any(|u| u.role == "admin");
    println!("  Any admin: {any_admin}");

    // =====================================================================
    // BONUS: method chaining beats nesting
    // =====================================================================
    println!("\n=== Method chaining: real example ===");

    // Get all unique tags from active admins, sorted
    // ❌ Probably 4+ levels of nesting with loops
    // ✅ 6 operations, zero nesting
    let mut unique_tags: Vec<&str> = users
        .iter()
        .filter(|u| u.active && u.role == "admin")
        .flat_map(|u| &u.tags)
        .map(|s| s.as_str())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    unique_tags.sort();
    println!("  Active admin tags: {unique_tags:?}");

    // =====================================================================
    // ITERTOOLS — extra weapons
    // =====================================================================
    // Add to Cargo.toml: itertools = "0.15"
    // Then use: use itertools::Itertools;
    //
    // Key functions that kill nested loops:
    //
    //   .chunks(n)       → group into chunks of n (replaces for i in 0..n step)
    //   .tuple_windows() → sliding window (replaces for i in 0..n-1)
    //   .group_by()      → group consecutive equal items
    //   .sorted()        → returns sorted Vec (replaces sort+collect)
    //   .unique()        → dedup (replaces HashSet dance above)
    //   .cartesian_product()  → all pairs from two iterators
    //   .combinations(n) → all n-element combinations
    //   .intersperse()   → insert separator between elements
    //   .counts()        → HashMap of frequencies
    //   .partition()     → split into two collections by predicate
    //   .flatten()       → flatten nested iterators (like flat_map but cleaner)
    //
    // Example with itertools (if you add it):
    //
    // use itertools::Itertools;
    //
    // let tags: Vec<_> = users.iter()
    //     .filter(|u| u.active)
    //     .flat_map(|u| &u.tags)
    //     .unique()
    //     .sorted()
    //     .collect();
    //
    // let freq: HashMap<&str, usize> = users.iter()
    //     .flat_map(|u| &u.tags)
    //     .map(|s| s.as_str())
    //     .counts();

    println!("\n=== Summary ===");
    println!("  Nested        → Iterator");
    println!("  for + if      → .filter().collect()");
    println!("  nested for    → .flat_map()");
    println!("  if-let chain  → .and_then().filter().map()");
    println!("  flag variable → .all() / .any()");
    println!("  accumulator   → .fold() / .sum()");
    println!("  group loop    → collect() into HashMap");
    println!("  itertools     → .unique(), .sorted(), .counts(), .cartesian_product()");
}
