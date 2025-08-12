# syntax=docker/dockerfile:1

# ---- Builder stage ---------------------------------------------------------
# Build the sigil-server binary against the pinned toolchain. We copy the whole
# workspace so the release build sees sigil-core and the embedded migrations.
FROM rust:1.90-slim AS builder

# musl is intentionally avoided to keep the ed25519/jsonwebtoken build simple;
# we ship on glibc (debian bookworm) in the runtime stage.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first to warm the dependency cache layer.
COPY Cargo.toml Cargo.lock* rust-toolchain.toml ./
COPY crates/sigil-core/Cargo.toml crates/sigil-core/Cargo.toml
COPY crates/sigil-server/Cargo.toml crates/sigil-server/Cargo.toml

# Create throwaway sources so `cargo build` can resolve and compile deps.
RUN mkdir -p crates/sigil-core/src crates/sigil-server/src \
    && echo 'pub fn _stub() {}' > crates/sigil-core/src/lib.rs \
    && echo 'fn main() {}' > crates/sigil-server/src/main.rs \
    && cargo build --release --locked -p sigil-server || true

# Now copy the real sources and migrations, then build for real.
COPY crates/ crates/
COPY migrations/ migrations/

# Bust the cached stub artifacts so the real sources are recompiled.
RUN touch crates/sigil-core/src/lib.rs crates/sigil-server/src/main.rs \
    && cargo build --release -p sigil-server \
    && strip target/release/sigil-server || true

# ---- Runtime stage ---------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# curl is required for the container HEALTHCHECK; ca-certificates for TLS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --user-group --home-dir /app --shell /usr/sbin/nologin sigil

WORKDIR /app

COPY --from=builder /build/target/release/sigil-server /usr/local/bin/sigil-server

USER sigil

ENV SIGIL_BIND=0.0.0.0:8080 \
    RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/sigil-server"]
