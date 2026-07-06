set dotenv-load

default:
    just --list

run:
    cargo run

test:
    cargo test

fmt:
    cargo fmt

lint:
    cargo clippy -- -D warnings

check:
    cargo check
