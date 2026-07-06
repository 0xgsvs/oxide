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

- Decision: `justfile`.
- Why: `just` is purpose-built for project tasks (dependencies, recipe args, shell completion). `mise` _can_ run tasks, but that is a secondary feature; its main job is toolchain version management. If you already use `mise` everywhere and want one less tool, `mise.toml` tasks are perfectly valid — just less common in Rust projects.

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
