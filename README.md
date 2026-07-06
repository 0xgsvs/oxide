# Oxide

A self-directed learning project: build a production-grade Rust backend API from scratch.

> **Rule #1:** You write the code. This repo contains only a roadmap, checkpoints, and pointers. If you copy a solution, you skip the learning.

## The Goal

By the end of this project you will have designed, built, deployed, and debugged a real backend service in Rust. You will not follow a tutorial step-by-step. You will make decisions, break things, read docs, fix them, and document why.

The domain is intentionally simple so the hard parts stand out: **a multi-tenant task/ticket tracker API**. Simple domain, hard infrastructure.

## What You Will Practice

Each requirement from the brief becomes a module you implement yourself:

| Requirement          | What You Will Build                                                 |
| -------------------- | ------------------------------------------------------------------- |
| Rust fundamentals    | Strong typing, ownership, error handling, traits in a real codebase |
| Async/Tokio          | Async handlers, spawn, channels, graceful shutdown                  |
| SQL performance      | Schema design, query plans, migrations, connection pooling          |
| REST + Microservices | Resource-oriented routes, service boundaries, health checks         |
| Redis caching        | Cache-aside and cache-invalidation patterns                         |
| JWT/OAuth + RBAC     | Login, token validation, role guards, permission checks             |
| Observability        | Structured logging, tracing, metrics, error propagation             |
| Performance          | Latency benchmarking, throughput testing, profiling                 |
| DevOps               | Docker, CI, deployment, environment config                          |

## Project Phases

### Phase 0: Foundation

- [ ] Set up the crate structure yourself. Decide: one crate or workspace?
- [ ] Choose an HTTP framework (Axum, Actix-web, or Rocket). Read its docs. Justify your choice in `docs/decisions.md`.
- [ ] Add a single `GET /health` route.
- [ ] Add structured logging with `tracing`.
- [ ] Write a small `justfile` or `Makefile` for `run`, `test`, `fmt`, `lint`.

**Checkpoint:** `curl http://localhost:3000/health` returns JSON. No database yet.

### Phase 1: Core API + Database

- [ ] Design the schema: `users`, `workspaces`, `tasks`, `comments`, `attachments`.
- [ ] Use `sqlx` or `diesel` or `sea-orm`. Pick one, justify it.
- [ ] Implement migrations.
- [ ] Build CRUD for `tasks`: `POST`, `GET`, `PATCH`, `DELETE`, `/tasks`.
- [ ] Add input validation.
- [ ] Use connection pooling.
- [ ] Write integration tests against a real test database.

**Checkpoint:** Full task CRUD works through `curl` and tests pass.

### Phase 2: Async + Concurrency

- [ ] Make handlers truly async. Identify what should not be `async`.
- [ ] Add background job: send email/webhook on task assignment. Use a channel or a task queue.
- [ ] Implement graceful shutdown: finish in-flight requests before exiting.
- [ ] Add rate limiting per user.

**Checkpoint:** Server shuts down cleanly under load. Background job runs in tests.

### Phase 3: Auth + Authorization

- [ ] Password hashing with `argon2`.
- [ ] JWT access tokens + refresh tokens.
- [ ] Middleware to extract user from token.
- [ ] Roles: `admin`, `manager`, `member`.
- [ ] Permission checks on tasks (owner, workspace member, admin).
- [ ] Optional: OAuth2 login via GitHub or Google.

**Checkpoint:** Unauthorized requests are rejected. Role tests cover happy and unhappy paths.

### Phase 4: Caching + Performance

- [ ] Add Redis. Cache task details and workspace metadata.
- [ ] Implement cache invalidation on updates.
- [ ] Add Redis-based rate limiter.
- [ ] Benchmark endpoints with `oha` or `wrk`.
- [ ] Profile and optimize one slow query using `EXPLAIN ANALYZE`.

**Checkpoint:** Cached endpoint is measurably faster. You can show before/after numbers.

### Phase 5: Observability + Resilience

- [ ] Add `tracing` spans and context propagation.
- [ ] Centralize errors with `thiserror` and a consistent JSON error shape.
- [ ] Add request IDs.
- [ ] Add metrics endpoint (`/metrics`) for Prometheus.
- [ ] Add timeout, retry, and circuit-breaker patterns where they make sense.

**Checkpoint:** You can trace a single request through logs by ID.

### Phase 6: Production + Deployment

- [ ] Dockerize the service with multi-stage build.
- [ ] Use environment-based config (`dotenvy` or env vars).
- [ ] Set up GitHub Actions CI: fmt, clippy, test, build image.
- [ ] Deploy to Fly.io, Render, or a small VPS.
- [ ] Add a `docs/runbook.md` for common incidents.

**Checkpoint:** Service is live and reachable over HTTPS.

## Suggested Directory Layout

This is a suggestion, not a rule. Reorganize as your design evolves.

```
oxide/
├── src/
│   ├── main.rs              # composition root
│   ├── config.rs            # env config
│   ├── routes/              # route handlers
│   ├── services/            # business logic
│   ├── models/              # domain types
│   ├── db/                  # migrations, pool, queries
│   ├── auth/                # jwt, password, roles
│   ├── cache/               # redis wrappers
│   ├── error.rs             # app error type
│   └── telemetry.rs         # logging/tracing setup
├── tests/
│   └── integration.rs
├── migrations/
├── docs/
│   ├── decisions.md         # why you chose each tool
│   ├── api.md               # API contract
│   └── runbook.md           # ops notes
├── Cargo.toml
├── Dockerfile
└── README.md
```

## Key Decisions to Document

Create `docs/decisions.md` and update it as you go. At minimum, answer:

1. Why this HTTP framework?
2. Why this database driver/ORM?
3. Sync or async database access?
4. How do you handle errors across layers?
5. What is your caching strategy and invalidation rule?
6. How do you manage secrets and environment config?

## Recommended Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
- [Zero To Production In Rust](https://www.zero2prod.com/) (strongly recommended)
- [SQLx docs](https://docs.rs/sqlx/latest/sqlx/) or your chosen ORM docs
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [High Performance Browser Networking](https://hpbn.co/)

## Learning Habits

- Read the error message fully before searching.
- Use `cargo clippy` and fix every warning.
- Write the test before the fix.
- When something works, ask: "what could break this?"
- When stuck for >30 minutes, write down what you tried, then ask.

## Current State

This repo is a blank Cargo project. Nothing is implemented yet. Start with Phase 0.
