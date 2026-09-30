# tpt-shipyard developer tasks (`cargo install just`, then `just <recipe>`).

default:
    @just --list

# Format, then run the full workspace test suite.
check:
    cargo fmt --all
    cargo clippy --workspace --all-targets --locked
    cargo test --workspace --locked

# Run the full workspace test suite only.
test:
    cargo test --workspace --locked

# Build the WASM bindings and regenerate www/pkg for the dashboard.
wasm:
    rustup target add wasm32-unknown-unknown
    cargo build -p tpt-yard-wasm --target wasm32-unknown-unknown --release --locked
    wasm-bindgen --out-dir www/pkg --target web \
        target/wasm32-unknown-unknown/release/tpt_yard_wasm.wasm
    @echo "Dashboard ready: open www/index.html (a static file server is enough)."

# Build the mdBook into docs/book/book.
book:
    mdbook build docs/book
    @echo "Book at docs/book/book/index.html"

# Regenerate every crate README/CHANGELOG from scripts/gen-crate-docs.py.
docs:
    python scripts/gen-crate-docs.py
