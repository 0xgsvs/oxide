//! Run: cargo run --example join-demo

use tokio::time::{Duration, sleep};

async fn fetch_user(id: u32) -> String {
    println!("  fetch_user({id}): starting...");
    sleep(Duration::from_millis(100)).await;
    println!("  fetch_user({id}): done");
    format!("user-{id}")
}

async fn fetch_posts(user_id: u32) -> Vec<String> {
    println!("  fetch_posts({user_id}): starting...");
    sleep(Duration::from_millis(150)).await;
    println!("  fetch_posts({user_id}): done");
    vec![
        format!("post-1 by {user_id}"),
        format!("post-2 by {user_id}"),
    ]
}

async fn fetch_comments(user_id: u32) -> Vec<String> {
    println!("  fetch_comments({user_id}): starting...");
    sleep(Duration::from_millis(120)).await;
    println!("  fetch_comments({user_id}): done");
    vec!["nice!".into(), "wow!".into()]
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let user_id = 42;

    println!("=== YOUR APPROACH: sequential .await (parks between each) ===");
    println!(
        "Timeline: start → fetch_user → PARK → done → fetch_posts → PARK → done → fetch_comments \
         → PARK → done"
    );
    println!("Total: 100 + 150 + 120 = 370ms\n");

    let start = std::time::Instant::now();
    let user = fetch_user(user_id).await; // 100ms, parks
    let posts = fetch_posts(user_id).await; // 150ms, parks (but user is ready!)
    let comments = fetch_comments(user_id).await; // 120ms, parks
    println!(
        "Sequential: user={user}, posts={:?}, comments={:?}",
        posts, comments
    );
    println!("({:?})\n", start.elapsed());

    // =========================================================================
    // What you want: "start all three, park only when ALL are waiting"
    // =========================================================================

    println!("=== WHAT YOU WANT: tokio::join! ===");
    println!("Timeline: start all three → fetch_user hits .await → CAN'T park yet!");
    println!("          fetch_posts hits .await → CAN'T park yet!");
    println!("          fetch_comments hits .await → ALL pending → park");
    println!("          fetch_user finishes → poll others → fetch_posts finishes");
    println!("          → fetch_comments finishes → all done!");
    println!("Total: max(100, 150, 120) = 150ms\n");

    let start = std::time::Instant::now();
    let (user, posts, comments) = tokio::join!(
        fetch_user(user_id),
        fetch_posts(user_id),
        fetch_comments(user_id),
    );
    println!(
        "Concurrent: user={user}, posts={:?}, comments={:?}",
        posts, comments
    );
    println!("({:?})", start.elapsed());

    // =========================================================================
    // How join! works internally
    // =========================================================================

    println!("\n=== HOW join! WORKS ===");

    // tokio::join! is a macro that generates something like:
    //
    //   Pin all three futures
    //   Loop:
    //       poll future_a
    //       if Ready → save result
    //       poll future_b
    //       if Ready → save result
    //       poll future_c
    //       if Ready → save result
    //       if all Ready → return (a, b, c)
    //       else → return Pending
    //
    // The key: it polls ALL futures before returning Pending.
    // All three get a chance to make progress on every poll cycle.

    println!("  join! polls ALL futures in a round-robin loop.");
    println!("  It only returns Pending when ALL futures return Pending.");
    println!("  That way, each future makes progress whenever ANY waker fires.");

    // =========================================================================
    // try_join! — same but with error handling
    // =========================================================================

    println!("\n=== BONUS: tokio::try_join! ===");
    println!("  Like join!, but returns Err if ANY future fails.");
    println!("  The remaining futures are dropped/cancelled.");

    // =========================================================================
    // When to use what
    // =========================================================================

    println!("\n=== DECISION GUIDE ===");
    println!("  Sequential .await:    B depends on A's result");
    println!("                           a = fetch_user().await");
    println!("                           b = fetch_posts(a.id).await");
    println!("  ");
    println!("  join!(a, b, c):       Independent work, all results needed");
    println!("                           join!(fetch_users(), fetch_config())");
    println!("  ");
    println!("  select!(a, b):        Race, first one wins, loser cancelled");
    println!("                           select! {{ biased;");
    println!("                               _ = timeout => ...");
    println!("                               val = fetch() => ...");
    println!("                           }}");
    println!("  ");
    println!("  spawn + await:        Long-lived background work");
    println!("                           tokio::spawn(heartbeat_loop())");
}
