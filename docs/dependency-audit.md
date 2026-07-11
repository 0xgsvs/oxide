# Dependency Audit: Oxide

| Icon | Meaning                                 |
| ---- | --------------------------------------- |
| ✓    | Used well                               |
| △    | Partial — useful features unused        |
| ○    | Minimal — barely scratching the surface |

---

### axum v0.8.8 — ✓

**Used:** Router, Json, extract::State, extract::FromRequestParts, middleware, response::IntoResponse, routing::{get, post}, http::{StatusCode, HeaderName, Request, Response}, body::Body, serve

**Verdict:** Core web framework, full usage of relevant APIs. No configurable features.

---

### tokio v1.52.3 — ✓

**Enabled:** rt, rt-multi-thread, macros, net, signal, sync, time, io-util, fs

**Unused (available):**

| Feature   | Why it matters                                       |
| --------- | ---------------------------------------------------- |
| `process` | Spawn child processes — not needed for an API server |

**Verdict:** Well-tuned. Every enabled feature is used directly or by axum/hyper/sqlx. `fs` is pulled by sqlx migration reading. `io-util` by hyper/axum I/O. Skipped `process` is correct — YAGNI.

---

### tracing v0.1.44 — ✓

**Used:** info!, error!, info_span!, instrument, Span

**Verdict:** Full usage of the macro/instrument surface. No configurable features.

---

### tracing-subscriber v0.3.23 — △

**Enabled:** fmt

**Unused (beneficial):**

| Feature/API  | Why it matters                                                                                                                                                                    |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registry`   | Enables `Registry` + `EnvFilter` for per-module log level control. Currently all tracing is fmt-based with no filtering — debug logs from dependencies (sqlx, axum) always print. |
| `json`       | Structured JSON logging for production log aggregation (Datadog, Loki, etc.).                                                                                                     |
| `env-filter` | Dynamic log filtering via `RUST_LOG` env var — lets ops teams turn up/down verbosity without a rebuild.                                                                           |

**Verdict:** Only `fmt` is enabled — no filtering, no structured output. Adding `env-filter` (enabled by `registry`) is a cheap win for operational debuggability.

---

### serde v1.0.228 — ✓

**Enabled:** derive

**Used:** Serialize, Deserialize derives on 7 types

**Verdict:** Exactly what derive is for. No useful unused features (alloc/std are for no-std/embedded).

---

### sqlx v0.9.0 — ✓

**Enabled:** runtime-tokio, tls-rustls, postgres, migrate

**Used:** PgPool, query!, query_as!, migrate!, Error

**Verdict:** Matched perfectly. Postgres-specific, TLS via rustls, compile-time checked queries, migrations. The `migrate` feature is well used.

---

### dotenvy v0.15.7 — ✓

**Used:** dotenv()

**Verdict:** Single-function crate used for its sole purpose. No configurable features.

---

### validator — △

**Enabled:** derive

**Unused (beneficial):**

| Feature/API | Why it matters                                                                                                                                                                       |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `email`     | Provides `#[validate(email)]` for `RegisterRequest.email` — currently only checking `is_empty`. Catches malformed emails at the API boundary instead of letting garbage into the DB. |

**Verdict:** Derive is used for `#[derive(Validate)]` on CreateTaskRequest and UpdateTaskRequest. The `email` validation feature would strengthen registration validation.

---

### argon2 v0.6.0-rc.8 — ✓

**Used:** Argon2, PasswordHasher::hash_password, PasswordVerifier::verify_password, PasswordHash

**Verdict:** Full usage of the password hashing API. No configurable features.

---

### jsonwebtoken v10.4.0 — ✓

**Enabled:** rust_crypto

**Unused (available):**

| Feature   | Why it matters                                                                                                             |
| --------- | -------------------------------------------------------------------------------------------------------------------------- |
| `use_pem` | Parse PEM-encoded RSA/EC keys. Currently using HMAC-SHA (symmetric secret). Only relevant if switching to asymmetric keys. |

**Verdict:** `rust_crypto` enables the default crypto backend. `use_pem` is YAGNI currently — HMAC is appropriate for a single-service app. Well used.

---

### chrono v0.4.45 — △

**Enabled:** serde

**Used:** Utc, Utc::now(), Duration::hours, .timestamp()

**Unused (beneficial):**

| Feature/API | Why it matters                                                                      |
| ----------- | ----------------------------------------------------------------------------------- |
| `clock`     | `Clock` trait for injecting mock time in tests. Currently tests can't control time. |

**Verdict:** `serde` enables serde support on DateTime types — needed for JWT claim serialization. No extra runtime features needed.

---

### serde_json v1.0.150 — ✓

**Used:** json!, from_str, to_string, Value

**Verdict:** The JSON operations workhorse. No misused features. The crate has no meaningful feature flags beyond std/default.

---

### redis v1.3.0 — △

**Enabled:** tokio-comp

**Used:** aio::MultiplexedConnection, AsyncCommands, Client, cmd::INCR, cmd::EXPIRE, set_ex, get, del

**Unused (beneficial):**

| Feature/API | Why it matters                                                                                                                                                    |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `streams`   | Redis Streams — enables persistent message queues. Could replace the in-memory mpsc channel for `TaskAssignedEvent` with a durable stream that survives restarts. |
| `script`    | Lua scripting (`EVAL`/`SCRIPT LOAD`) — for atomic multi-key operations without race conditions. Could simplify the versioned cache invalidation pattern.          |
| `acl`       | Redis 6+ ACL commands — useful if Redis is shared across services.                                                                                                |

**Verdict:** Current usage is basic key-value + INCR/EXPIRE for rate limiting and cache invalidation. The `streams` feature is the most actionable — it would make task assignment events durable.

---

### thiserror v2.0.18 — ✓

**Used:** #[derive(Error)]

**Verdict:** The derive macro is the crate. No configurable features.

---

### tower-http v0.7.0 — ✓

**Enabled:** trace, request-id, catch-panic, sensitive-headers, normalize-path, compression-gzip, timeout

**Unused (beneficial):**

| Feature | Why it matters                                                                                                               |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `limit` | Request body size limit (`RequestBodyLimitLayer`) — prevents large payload attacks. Currently no limit on POST/PATCH bodies. |

**Verdict:** The enabled set is well chosen for an API server — tracing, request tracing, panic safety, path normalization, gzip compression, and timeout. Skipped `limit` is the most notable gap (security hardening). Skipped `compression-br` is reasonable — brotli gives ~20% better compression but adds 200KB+ to binary and more CPU on compress.

---

### prometheus v0.14.0 — ○

**Enabled:** process

**Used:** CounterVec, HistogramVec, TextEncoder, register_counter_vec!, register_histogram_vec!, gather

**Verdict:** Two metric families (request count + duration histogram) is a good start. Unused `protobuf` feature (protobuf exposition format instead of text) is fine — text is standard.

**Potential additions not needing features:**

- `Gauge` for active connections / in-flight requests
- Request size histogram for insight into payload patterns

These don't need new features, just new metric registrations.

---

### utoipa v5.5.0 — △

**Enabled:** axum_extras, chrono

**Used:** OpenApi derive, ToSchema, IntoParams, Modify trait, openapi::security (HttpAuthScheme, HttpBuilder, SecurityScheme)

**Unused (beneficial):**

| Feature/API                          | Why it matters                                                                                        |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| `utoipa-swagger-ui` (separate crate) | Self-hosted Swagger UI instead of loading from CDN. Works offline, no external dependency at runtime. |

**Verdict:** OpenAPI spec generation is fully leveraged — 6 endpoints documented, 6 schema types, Bearer auth, tagging. The `axum_extras` is well used. `utoipa-swagger-ui` is a nice-to-have for air-gapped deployments.

---

## Priority Action Board

| Pri    | Change                                                                                         | Dep                | LoC | Impact                                                                         |
| ------ | ---------------------------------------------------------------------------------------------- | ------------------ | --- | ------------------------------------------------------------------------------ |
| **P1** | Enable `registry` + `env-filter` on tracing-subscriber                                         | tracing-subscriber | ±2  | Production debuggability: control log verbosity via `RUST_LOG` without rebuild |
| **P2** | Enable `limit` feature on tower-http, add `RequestBodyLimitLayer` (e.g. 1MB)                   | tower-http         | +1  | Security: prevent large payload DoS                                            |
| **P2** | Add `#[validate(email)]` to `RegisterRequest.email`                                            | validator          | +1  | Catch malformed emails at API boundary                                         |
| **P3** | Enable `streams` on redis — replace mpsc channel with Redis Streams for task-assignment events | redis              | ~20 | Persistence: task assignments survive process restart                          |
| **P3** | Add Gauge + request-size histogram to metrics                                                  | prometheus         | +10 | Observability: active connections and payload size distribution                |
| **P4** | Add `utoipa-swagger-ui` crate for self-hosted docs UI                                          | utoipa             | +3  | Remove CDN dependency, offline-capable                                         |
| **P4** | Enable `script` on redis for atomic Lua-based cache operations                                 | redis              | ~10 | Simplify invalidation logic with Lua scripts                                   |
