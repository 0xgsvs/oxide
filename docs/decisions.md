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

- Decision:
- Why:

### Migration tool

- Decision:
- Why:
