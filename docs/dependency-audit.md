# Dependency Audit: Oxide

> Generated 2026-07-08 — analyzes whether each dependency's feature surface is being squeezed.

## Legend

| Icon | Meaning                                                    |
| ---- | ---------------------------------------------------------- |
| ✓    | Used well — full value extracted                           |
| △    | Partial — using core, leaving useful features on the table |
| ○    | Minimal — barely scratching the surface                    |
| —    | Single-purpose crate, no meaningful unused surface         |

---

## 1. `axum` 0.8.9 — △ Partial

**Used:**
`Json`, `Router`, `State`, `extract::{Path, Query, FromRequestParts}`,
`http::{StatusCode, Request, Response, HeaderName}`,
`middleware::{self, from_fn_with_state, Next}`,
`response::{IntoResponse, Html}`, `routing::{get, post, patch, delete}`,
`serve`, `body::Body`

**Unused (potentially useful):**

| Feature                      | What it replaces / why                                                     |
| ---------------------------- | -------------------------------------------------------------------------- |
| `response::Sse`              | Real-time task notifications (instead of polling or in-process mpsc)       |
| `response::Redirect`         | After-login redirects                                                      |
| `routing::any()`             | Catch-all / fallback routes                                                |
| `middleware::from_extractor` | Declarative middleware — cleaner than `from_fn_with_state` for simple auth |
| `extract::Multipart`         | File uploads (schema has `attachments` table)                              |
| `extract::Form`              | URL-encoded form handling                                                  |
| `error_handling` module      | Already covered by custom `AppError`                                       |

**Verdict:** Core usage is solid. `Sse` and `Multipart` are the main upgrades when features demand them.

---

## 2. `tokio` 1.52.3 — △ Partial

**Used (features):**
`rt`, `rt-multi-thread`, `macros`, `net`, `signal`, `sync` (mpsc), `time`, `io-util`, `fs`

**Used (APIs):**
`tokio::spawn`, `tokio::select!`, `#[tokio::main]`,
`TcpListener`, `ctrl_c`, `unix::signal`, `mpsc::{channel, Sender, Receiver}`

**Unused (potentially useful):**

| Feature / API                       | Why                                                                                  |
| ----------------------------------- | ------------------------------------------------------------------------------------ |
| `sync::broadcast`                   | Fan-out task notifications (mpsc is 1:1, broadcast would support multiple listeners) |
| `sync::watch`                       | Config reload signals, cache invalidation                                            |
| `sync::Mutex` (async)               | Mutable shared state held across `.await`                                            |
| `sync::RwLock`                      | Read-heavy shared state                                                              |
| `sync::Semaphore`                   | Concurrency cap (max N concurrent DB writes)                                         |
| `task::JoinSet`                     | Structured dynamic task management                                                   |
| `task::spawn_blocking`              | **Critical** — `argon2` hashing/verifying currently blocks async threads             |
| `time::interval`                    | Periodic health checks, housekeeping                                                 |
| `io::{AsyncReadExt, AsyncWriteExt}` | Raw async I/O if needed                                                              |

**Verdict:** `spawn_blocking` for password hashing is the most impactful miss — blocking the async runtime with CPU-bound work hurts tail latency.

---

## 3. `tower-http` 0.7.0 — ○ Minimal (biggest gap)

**Used:** `trace`, `request-id`

**Unused (potentially useful):**

| Feature             | Effort                 | Impact                                                       |
| ------------------- | ---------------------- | ------------------------------------------------------------ |
| `cors`              | 1 feature flag + 5 LoC | **P1** — needed for separate frontend origin                 |
| `compression-gzip`  | 1 flag + 1 layer       | **P1** — smaller JSON payloads                               |
| `timeout`           | 1 flag + 1 layer       | **P1** — guard against slow requests                         |
| `catch-panic`       | 1 flag + 1 layer       | **P1** — convert panics to 500s                              |
| `sensitive-headers` | 1 flag + 1 layer       | **P1** — mask `Authorization` in logs                        |
| `set-header`        | 1 flag + 1 layer       | security headers (`X-Content-Type-Options`, etc.)            |
| `limit` (body size) | 1 flag + 1 layer       | protect against large payloads                               |
| `normalize-path`    | 1 flag + 1 layer       | trailing-slash normalization                                 |
| `propagate-header`  | 1 flag                 | forward request-id to downstream                             |
| `metrics`           | 1 flag                 | could replace custom `metrics.rs` with tower-http's built-in |

**Verdict:** Biggest ROI in the entire audit. Currently using 2 of ~24 middleware modules. Adding 5 feature flags (+~10 lines of app code) gives CORS, compression, timeout, panic safety, and log hygiene.

---

## 4. `sqlx` 0.9.0 — △ Partial

**Used (features):** `runtime-tokio`, `tls-rustls`, `postgres`, `migrate`

**Used (APIs):**
`PgPool`, `PgPool::connect`, `query!`, `query_as!`, `fetch_one`, `fetch_optional`, `fetch_all`, `execute`

**Unused (potentially useful):**

| Feature / API                                   | Why                                                                                       |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `Transaction` (`pool.begin()` / `txn.commit()`) | **P1 correctness** — no endpoint uses transactions. Create/update tasks should be atomic. |
| `PoolOptions`                                   | Tune max connections, timeouts, etc.                                                      |
| `Pool::try_acquire`                             | Non-blocking pool check                                                                   |
| `Acquire` trait                                 | Borrow a connection for multiple operations                                               |
| `QueryBuilder`                                  | Dynamic query construction for filtered task list                                         |
| `migrate!` macro                                | Embed migrations in binary                                                                |
| `query_as_unchecked!`                           | Faster when schema is trusted                                                             |

**Verdict:** Missing transactions is a correctness gap, not a feature gap. Every mutation endpoint should wrap in a transaction.

---

## 5. `redis` 1.3.0 — ○ Minimal

**Used:**
`Client::open`, `get_multiplexed_async_connection`, `AsyncCommands::{get, del}`,
`redis::cmd("SET")`, `redis::cmd("INCR")`, `redis::cmd("EXPIRE")`

**Unused (potentially useful):**

| Feature / API                            | Why                                                                      |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| `redis::pipe()`                          | **P2** — batch SET + EXPIRE into 1 round trip instead of 2               |
| `redis::transaction()`                   | Atomic multi-key operations                                              |
| Pub/Sub                                  | Real-time task notification (replace in-process mpsc with cross-process) |
| `redis::json`                            | Store/retrieve structured cache data without serde serialization         |
| `ClusterClient`                          | Redis cluster connectivity                                               |
| Connection pooling (`ConnectionManager`) | Managed reconnect                                                        |

**Verdict:** Pipelines alone cut Redis round trips in half for cache writes. The custom `set_string` does SET then EXPIRE as two commands.

---

## 6. `chrono` 0.4.45 — ○ Minimal

**Used:** `Duration`, `Utc` (for JWT `iat`/`exp`)

**Unused (potentially useful):**

| Feature                      | Why                                                                                      |
| ---------------------------- | ---------------------------------------------------------------------------------------- |
| `DateTime<Utc>` model fields | **P3** — `Task`, `User` have `TIMESTAMPTZ` columns in DB but Rust models don't read them |
| `serde` feature              | Enabled but never used on any chrono type                                                |
| `NaiveDateTime`              | Simpler timezone-naive variant                                                           |
| `Local`                      | OS-local time display                                                                    |
| `format` / `parse`           | Date string formatting                                                                   |
| `TimeDelta`                  | Newtype for `Duration`                                                                   |

**Verdict:** Adding `created_at: DateTime<Utc>` / `updated_at: DateTime<Utc>` to the `Task` model unlocks user-facing timestamps with zero new dependencies.

---

## 7. `serde` 1.0.228 — ✓ Used well

**Used:** `Deserialize`, `Serialize` (derive macros)

**Cosmetic additions (no new features):**

| Attribute                                           | Benefit                   |
| --------------------------------------------------- | ------------------------- |
| `#[serde(rename_all = "camelCase")]`                | JS-friendly API responses |
| `#[serde(skip_serializing_if = "Option::is_none")]` | Tighter JSON              |
| `#[serde(default)]`                                 | Graceful missing fields   |

**Verdict:** Core value extracted. Attributes are polish.

---

## 8. `utoipa` 5.5.0 — △ Partial

**Used:**
`OpenApi`, `ToSchema`, `#[utoipa::path]`, `IntoParams`,
`openapi::security::{HttpBuilder, HttpAuthScheme, SecurityScheme}`, `Modify`

**Unused (potentially useful):**

| Feature                     | Why                                                                             |
| --------------------------- | ------------------------------------------------------------------------------- |
| `utoipa-swagger-ui` crate   | Self-host Swagger UI (currently loading from CDN — one more dependency to fail) |
| `ToResponse`                | Shared response schemas                                                         |
| `utoipa::openapi::response` | Custom response documentation                                                   |

**Verdict:** Consider `utoipa-swagger-ui` to eliminate the CDN dependency for the docs page.

---

## 9. `prometheus` 0.14.0 — △ Partial

**Used:**
`CounterVec`, `HistogramVec`, `TextEncoder`, `register_counter_vec!`,
`register_histogram_vec!`, `gather`

**Unused (potentially useful):**

| Feature                       | Why                                               |
| ----------------------------- | ------------------------------------------------- |
| `Gauge`                       | Track in-flight requests, pool size, queue depth  |
| `process` feature             | **Already enabled** — collects RSS, CPU, FD count |
| `labels!` macro               | Cleaner label construction                        |
| `register_counter!` (non-vec) | When no labels needed                             |
| `push` feature                | Pushgateway integration                           |

**Verdict:** Adding a `Gauge` for concurrent requests is a few lines and gives useful operational insight.

---

## 10. `validator` 0.20.0 — △ Partial

**Used:** `Validate` derive, `ValidationErrors`

**Unused (potentially useful):**

| Rule                          | Where                                                        |
| ----------------------------- | ------------------------------------------------------------ |
| `#[validate(email)]`          | `RegisterRequest.email` — currently only checks `is_empty()` |
| `#[validate(url)]`            | Future task-link fields                                      |
| `#[validate(custom = "...")]` | Domain-specific validation                                   |
| `#[validate(nested)]`         | Nested struct validation                                     |

**Verdict:** `#[validate(email)]` on `RegisterRequest.email` is a one-line improvement for input quality.

---

## 11. `argon2` 0.6.0-rc.8 — ✓ Used well

**Used:**
`Argon2::default()`, `PasswordHasher::hash_password`, `PasswordVerifier::verify_password`,
`PasswordHash::new`

**Verdict:** Full flow used. Only concern is that it runs on async threads without `spawn_blocking`.

---

## 12. `jsonwebtoken` 10.4.0 — △ Partial

**Used:** `encode`, `decode`, `Header`, `Validation::default`, `DecodingKey`, `EncodingKey`

**Unused (potentially useful):**

| Feature                            | Why                                            |
| ---------------------------------- | ---------------------------------------------- |
| `Validation { leeway: 60, .. }`    | **P2** — clock skew tolerance between services |
| `Validation::required_spec_claims` | Explicit claim requirements                    |
| `Algorithm::HS512`                 | Stronger signing than HS256 (default)          |
| Custom `kid` header                | Key rotation support                           |

**Verdict:** Adding 60s leeway is a two-line production hardening.

---

## 13. `tracing` 0.1.44 / `tracing-subscriber` 0.3.23 — ✓ Used well

**Used:**
`instrument`, `info`, `info_span`, `error`, `tracing_subscriber::fmt::init`

**Unused (potentially useful):**

| Feature                         | Why                                                   |
| ------------------------------- | ----------------------------------------------------- |
| `tracing_subscriber::EnvFilter` | **P3** — runtime log-level control (`RUST_LOG=debug`) |
| JSON layer                      | Structured logging for production log ingestion       |
| `warn!`, `debug!`, `trace!`     | Granular log levels                                   |

**Verdict:** `EnvFilter` is the standard pattern — currently everything is `info` with no filtering.

---

## 14. `thiserror` 2.0.18 — ✓ Used well

**Used:** `#[derive(Error)]`, `#[error("...")]`, `#[from]`

**Verdict:** Full value.

---

## 15. `serde_json` 1.0.150 — ✓ Used well

**Used:** `json!`, `from_str`, `to_string`, `Value`

**Verdict:** Serializer — no meaningful unused API surface.

---

## 16. `dotenvy` 0.15.7 — ✓ Used

**Used:** `dotenv()`

**Verdict:** Single-purpose crate, fully utilized.

---

## 17. `tower` 0.5.3 (dev) — ✓ Used well

**Used:** `ServiceExt::oneshot` in integration tests.

**Verdict:** Its job is test utilities. Done.

---

## Priority Action Board

| Pri | Change                                                                               | Dep                  | Lines                  | Impact                          |
| --- | ------------------------------------------------------------------------------------ | -------------------- | ---------------------- | ------------------------------- |
| P1  | Add `cors`, `compression-gzip`, `timeout`, `catch-panic`, `sensitive-headers` layers | `tower-http`         | ~10 app + 5 Cargo.toml | Security + perf + resilience    |
| P1  | Wrap mutations in `pool.begin()` / `txn.commit()`                                    | `sqlx`               | ~8 per endpoint        | Correctness (no partial writes) |
| P2  | Use `spawn_blocking` for password hash/verify                                        | `tokio`              | ~5                     | Async runtime health            |
| P2  | Use `redis::pipe()` for atomic SET+EXPIRE                                            | `redis`              | ~5                     | 50% fewer Redis round trips     |
| P2  | Add `leeway: 60` to JWT `Validation`                                                 | `jsonwebtoken`       | 2                      | Clock skew tolerance            |
| P2  | Add `#[validate(email)]` to register request                                         | `validator`          | 1                      | Input quality                   |
| P3  | Add `created_at`/`updated_at` (chrono) to Task model                                 | `chrono`             | ~8                     | User-facing timestamps          |
| P3  | Add `EnvFilter`                                                                      | `tracing-subscriber` | ~3                     | Runtime log control             |
| P3  | Add `Gauge` for in-flight requests                                                   | `prometheus`         | ~5                     | Observability                   |
| P4  | Swap CDN swagger for `utoipa-swagger-ui`                                             | `utoipa`             | ~5                     | Remove CDN dependency           |

---

## Files with `ponytail:` markers

None found in `src/`. No deliberate shortcuts are documented.
