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

- [x] Set up the crate structure — single crate, not a workspace.
- [x] Choose an HTTP framework: **Axum** (Tokio-native, Tower ecosystem).
- [x] Add a single `GET /health` route.
- [x] Add structured logging with `tracing`.
- [x] Write a `mise.toml` with `dev`, `test`, `fmt`, `lint`, `check`, `ready`, `ci` tasks.

**Checkpoint:** `xh http://localhost:3000/health` returns JSON. No database yet.

### Phase 1: Core API + Database

- [x] Design the schema: `users`, `workspaces`, `tasks`, `comments`, `attachments`.
- [x] Use **SQLx** with PostgreSQL (async, compile-time checked queries).
- [x] Implement reversible migrations (`.up.sql` / `.down.sql`).
- [x] Build CRUD for `tasks`: `POST`, `GET`, `PATCH`, `DELETE`, `/tasks`.
- [x] Add input validation with `validator` crate.
- [x] Use connection pooling (`PgPool`).
- [x] Write integration tests with `#[sqlx::test]` (isolated test DB per test).

**Checkpoint:** Full task CRUD works through `xh` and tests pass.

### Phase 2: Async + Concurrency

- [x] Handlers are async. CPU-bound work identified (deferred).
- [x] Add background job: `tokio::sync::mpsc` channel for task assignment events.
- [x] Implement graceful shutdown on `SIGINT` / `SIGTERM`.
- [x] Add rate limiting with `tower_governor` (global key, per-second/burst).

**Checkpoint:** Server shuts down cleanly under load. Background job runs in tests.

### Phase 3: Auth + Authorization

- [x] Password hashing with **Argon2** (automatic salt, default params).
- [x] JWT access tokens (24h expiry) with `jsonwebtoken` + `rust_crypto`.
- [x] `AuthUser` extractor via `FromRequestParts` — validates Bearer token.
- [x] Roles: `admin`, `manager`, `member` (stored in JWT claims, not enforced at endpoint level yet).
- [ ] OAuth2 login via GitHub or Google (optional).

**Checkpoint:** Unauthorized requests return 401. Auth tests cover register, login, wrong password.

### Phase 4: Caching + Performance

- [x] Add Redis (`redis` crate, `MultiplexedConnection`).
- [x] Cache task details (`task:{id}`, 60s TTL) and paginated lists (`tasks:list:{limit}:{offset}`, 30s TTL).
- [x] Implement cache invalidation on create, update, delete.
- [x] Docker Compose (`compose.yml`) for PostgreSQL + Redis.
- [x] Benchmark endpoints with `oha` (installed via mise).
- [ ] Profile and optimize one slow query using `EXPLAIN ANALYZE`.

**Checkpoint:** Cached endpoint is measurably faster. Cache invalidation logic verified in tests.

### Phase 5: Observability + Resilience

- [x] Add `tracing` spans with `#[instrument]` on handlers.
- [x] Add `TraceLayer` for request-level spans (method, URI, request ID).
- [x] Centralize errors with `thiserror` / `AppError` enum, consistent JSON responses.
- [x] Add request IDs (`x-request-id` UUID via `SetRequestIdLayer`).
- [x] Add metrics endpoint (`/metrics`) with `prometheus` crate — counters, latency histograms, process stats.
- [ ] Add timeout, retry, and circuit-breaker patterns (deferred until needed).

**Checkpoint:** You can trace a single request through logs by ID. `/metrics` exposes HTTP and process metrics.

### Phase 6: Production + Deployment

- [x] Set up GitHub Actions CI: fmt, clippy, test.
- [x] Add a `docs/runbook.md` for common incidents.
- [ ] Dockerize the service with multi-stage build (pending Dockerfile exercise).

**Checkpoint:** Service is live and reachable over HTTPS.

## Current State

This repo has completed Phases 0–5. The project is ready for Phase 6 (deployment).

- Task CRUD API with PostgreSQL, SQLx, and Redis cache
- JWT authentication with Argon2 password hashing
- Rate limiting, graceful shutdown, background job worker
- Centralized error handling with structured JSON responses
- Prometheus metrics at `/metrics`
- Tracing with request IDs
- Integration tests (6 tests, all passing)
- Infrastructure via Docker Compose

## Directory Layout

```
oxide/
├── src/
│   ├── main.rs              # composition root
│   ├── lib.rs               # create_app() + Tower layers
│   ├── config.rs            # env config (DATABASE_URL, JWT_SECRET, REDIS_URL)
│   ├── auth.rs              # JWT, AuthUser extractor, register/login handlers
│   ├── cache.rs             # Redis helpers (get/set/del)
│   ├── db.rs                # PgPool creation
│   ├── error.rs             # AppError enum + IntoResponse
│   ├── metrics.rs           # Prometheus counters + histograms
│   ├── models/              # Models will be extracted from models.rs
│   ├── routes/              # route handlers
│   │   ├── mod.rs           # health endpoint
│   │   └── tasks.rs         # task CRUD handlers
│   └── models.rs            # Task, CreateTaskRequest, UpdateTaskRequest, TaskAssignedEvent
├── tests/
│   └── integration.rs       # 6 integration tests
├── migrations/              # SQLx reversible migrations
│   ├── ..._initial_schema.up.sql
│   └── ..._initial_schema.down.sql
├── docs/
│   ├── decisions.md         # why each tool was chosen
│   └── database.md          # database and redis setup guide
├── Cargo.toml
├── compose.yml              # PostgreSQL (port 5433) + Redis
├── docs/
│   ├── decisions.md         # why each tool was chosen
│   ├── database.md          # database and redis setup guide
│   └── runbook.md           # common issues and fixes
├── .env                     # DATABASE_URL, JWT_SECRET, REDIS_URL
├── .gitignore
├── .github/
│   └── workflows/
│       └── ci.yml           # GitHub Actions
├── mise.toml              # task runner config
└── README.md
```

## Starting the Stack

```bash
# Start infrastructure
docker compose up -d

# Run migrations
sqlx migrate run

# Start the app
mise run dev
```

Testing:

```bash
# Register a user and get a token
xh post http://localhost:3000/auth/register email="user@example.com" password="..."

# Use the token for task operations
xh get http://localhost:3000/tasks "Authorization:Bearer $TOKEN"

# View metrics
xh get http://localhost:3000/metrics
```

## Infrastructure

| Service              | Port | Start command          |
| -------------------- | ---- | ---------------------- |
| App                  | 3000 | `mise run dev`         |
| PostgreSQL (compose) | 5433 | `docker compose up -d` |
| PostgreSQL (local)   | 5432 | system service         |
| Redis                | 6379 | `docker compose up -d` |

## Beyond Phase 6

After deployment, the next natural extensions:

| Extension                            | What it adds                                                                                  | When to add                     |
| ------------------------------------ | --------------------------------------------------------------------------------------------- | ------------------------------- |
| **Message queue** (Kafka / RabbitMQ) | Replace in-process mpsc channel with a distributed job queue. Enables multiple app instances. | After multi-instance deployment |
| **Prometheus scraper**               | Add Prometheus server to compose stack to scrape `/metrics`.                                  | After Phase 6                   |
| **Grafana dashboards**               | Visualize metrics (latency, error rate, throughput).                                          | After Prometheus scraper        |
| **Per-user rate limiting**           | Replace `GlobalKeyExtractor` with a per-user key extractor using JWT claims.                  | After auth is stable            |
| **Refresh tokens**                   | Add `/auth/refresh` endpoint for rotating JWTs without re-login.                              | After basic auth is tested      |
| **OAuth2**                           | GitHub / Google login via `oauth2` crate.                                                     | After JWT auth is stable        |

## Key Decisions

All architecture decisions are documented in [`docs/decisions.md`](docs/decisions.md), including:

1. Why Axum over Actix/Rocket
2. Why SQLx over Diesel/Sea-ORM
3. Why Argon2 for passwords
4. Why Redis cache-aside over write-through
5. Why thiserror over snafu
6. Why GlobalKeyExtractor for rate limiting (per-user planned after auth)

## Recommended Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
- [Zero To Production In Rust](https://www.zero2prod.com/) (strongly recommended)
- [SQLx docs](https://docs.rs/sqlx/latest/sqlx/)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [High Performance Browser Networking](https://hpbn.co/)

## Learning Habits

- Read the error message fully before searching.
- Use `cargo clippy` and fix every warning.
- Write the test before the fix.
- When something works, ask: "what could break this?"
- When stuck for >30 minutes, write down what you tried, then ask.
