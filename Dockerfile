# Multi-stage build for the tpt-shipyard CLI (review 7F).
FROM rust:1.98-slim AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY examples ./examples
COPY benches ./benches
RUN cargo build --release -p tpt-yard-cli --locked

FROM debian:bookworm-slim
COPY --from=build /build/target/release/tpt-yard /usr/local/bin/tpt-yard
# The hull manifests the plan command consumes by default.
COPY test-data /data/test-data
WORKDIR /data
ENTRYPOINT ["tpt-yard"]
CMD ["--help"]
