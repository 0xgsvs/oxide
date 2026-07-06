set dotenv-load

default:
    just --list

run:
    cargo run

run_release:
    cargo run --release

test:
    cargo nextest run --no-tests=pass

fmt:
    cargo fmt

lint:
    cargo clippy -- -D warnings

check:
    cargo check
