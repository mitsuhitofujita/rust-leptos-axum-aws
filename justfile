default:
    @just --list

# Frontend dev server on http://localhost:8080 (proxies /api to the API server).
dev-web:
    trunk serve

# API dev server on http://localhost:3000.
dev-api:
    cargo run -p server

# Build everything the way it would be shipped.
build:
    cargo build --workspace --release
    trunk build --release

check:
    cargo check --workspace
    cargo check -p app --target wasm32-unknown-unknown

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

test:
    cargo test --workspace

clean:
    cargo clean
    rm -rf dist
