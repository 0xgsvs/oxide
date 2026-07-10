# Architecture Decisions

Record each significant decision here. One or two sentences is enough. The goal is to practice reasoning, not write a formal ADR.

## Phase 0

### One crate or workspace?

- Decision: One crate for now.
- Why: The domain is a single API service. A workspace adds friction before there is a real boundary like a separate worker binary or shared library. Revisit when the project splits.

### HTTP framework

- Decision: Axum.
- Why: Native Tokio integration, Tower middleware ecosystem, and modern extractor-based routing. Most current Rust backend learning material and production code uses it.

### Logging / tracing

- Decision: `tracing` + `tracing-subscriber` with the `fmt` subscriber.
- Why: `tracing` is the standard for async-aware structured logging in Rust and integrates cleanly with Tokio and Axum.

### Task runner

- Decision: `mise` tasks in `mise.toml`.
- Why: mise tasks have matured into a full task-runner system — dependencies, typed arguments with autocomplete, aliases, per-task env vars, sandboxing, interactive picker, and dep graph visualization. Since mise is already the toolchain manager (`[tools]`), using it for tasks removes `justfile` entirely: one file, one tool, zero loss. `just` was the original pick when mise's task system was still secondary, but it no longer is.

### HTTP client

- Decision: `xh`.
- Why: Simpler syntax and better output formatting than `curl` for manual API testing.

### Test runner

- Decision: `cargo nextest`.
- Why: Faster, clearer output, and better handling of async tests than the default `cargo test`.

## Phase 1

### Database driver / ORM

- Decision: SQLx with PostgreSQL.
- Why: Async-native, compile-time checked queries, and forces learning real SQL instead of hiding it behind an ORM DSL.

### Migration tool

- Decision: `sqlx-cli`.
- Why: Native companion to SQLx. Migrations are plain SQL files in `migrations/`, version-controlled and runnable offline.

### Input validation

- Decision: `validator` with explicit `validate()` calls in handlers.
- Why: Standard Rust validation library with derive macros. Kept explicit for now instead of an Axum extractor wrapper so the learning path is clear. Error responses will be centralized later.

### Integration testing

- Decision: `#[sqlx::test]` + `tower::ServiceExt::oneshot`.
- Why: `#[sqlx::test]` creates an isolated test database per test and runs migrations automatically. `ServiceExt::oneshot` tests the Axum router without binding a real port.

## Phase 2

### Async runtime patterns

- Decision: Use Tokio's `mpsc` channel for background jobs and `tokio::signal` for graceful shutdown.
- Why: Native Tokio primitives. No extra dependencies for the core async patterns.

### Rate limiting

- Decision: `tower_governor` with `GlobalKeyExtractor` for now.
- Why: Easy Tower integration. Per-IP/key extractors require connect info that is hard to provide in tests and not meaningful without authentication. Will switch to per-user limiting after Phase 3 auth.

## Phase 3

### Authentication approach

- Decision: JWT with stateless tokens. Password hashing with Argon2. Auth via `FromRequestParts` extractor.
- Why: JWT avoids server-side session storage. Argon2 is the standard password hashing algorithm.

### Password hashing

- Decision: `argon2` crate with default features.
- Why: Pure Rust implementation of Argon2id, salt generated automatically.

### JWT

- Decision: `jsonwebtoken` with `rust_crypto` feature.
- Why: Standard Rust JWT library. Tokens expire after 24 hours.

### Authorization

- Decision: Use Axum's `FromRequestParts` extractor (`AuthUser`) instead of a Tower middleware layer.
- Why: Handler-level control means routes like `/auth/register` are unauthenticated while `/tasks` require a valid token. No complex middleware chains.

### Role-based access control

- Not yet implemented at endpoint level. The `Claims` struct carries the role. Endpoint-level enforcement will be added when role-specific logic is needed.

## Phase 4

### Caching approach

- Decision: Redis with cache-aside pattern.
- Why: Task reads are frequent and cheap to cache. Cache-aside is simple: read from cache first, fall back to DB, then populate cache. Invalidation on write (create/update/delete) keeps data fresh.

### Redis library

- Decision: `redis` crate v1.3.0 with `tokio-comp` feature.
- Why: Standard Rust Redis driver with direct async support via `MultiplexedConnection`.

### Cache keys & TTL

- `task:{id}`: single task, TTL 60s, invalidated on PATCH/DELETE.
- `tasks:list:{uid}:{version}:{limit}:{offset}`: paginated list per user, TTL 30s.
  - **Versioned key pattern**: each user has a counter (`tasks:list:version:{uid}`) stored in Redis.
    On any task write (create/update/delete), we `INCR` that counter.
    The next list read builds the cache key with the new version → cache miss → fresh DB query →
    cache populated. Old entries become unreachable and expire via their 30s TTL.
  - `INCR` is O(1) and atomic — no race conditions, no scan-and-delete needed.

### Infrastructure

- Decision: Docker Compose (`compose.yml`) for both PostgreSQL and Redis.
- Why: Single `docker compose up` starts the full stack. Redis via compose, Postgres via compose (optional, profile `full`).

### Benchmarking

- Not yet performed. Requires `oha`, `wrk`, or `hyperfine` to compare latency before/after.
