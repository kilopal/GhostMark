# ─────────────────────────────────────────────────────────────
# GhostMark — Multi-stage Docker build
# Stage 1: Build the Rust binary
# Stage 2: Copy only the binary into a minimal runtime image
# ─────────────────────────────────────────────────────────────

# ── Build stage ──────────────────────────────────────────────
FROM rust:1.88-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/Cargo.toml
COPY cli/Cargo.toml cli/Cargo.toml
COPY wasm/Cargo.toml wasm/Cargo.toml

# Create dummy source files so cargo can fetch and cache dependencies
RUN mkdir -p core/src cli/src wasm/src \
    && echo "pub fn dummy() {}" > core/src/lib.rs \
    && echo "fn main() {}" > cli/src/main.rs \
    && echo "pub fn dummy() {}" > wasm/src/lib.rs \
    && cargo build --release -p ghostmark 2>/dev/null || true

# Copy actual source code
COPY core/ core/
COPY cli/ cli/
COPY wasm/ wasm/

# Touch source files to invalidate the dummy build cache
RUN touch core/src/lib.rs cli/src/main.rs

# Build the real binary
RUN cargo build --release -p ghostmark

# ── Runtime stage ────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd --create-home --shell /bin/bash ghostmark

WORKDIR /home/ghostmark

# Copy the compiled binary from the build stage
COPY --from=builder /app/target/release/ghostmark /usr/local/bin/ghostmark

# Switch to non-root user
USER ghostmark

# Expose the default proxy port
EXPOSE 8080

# Health check against the /health endpoint
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -sf http://localhost:8080/health || exit 1

# Default command: start the HTTP proxy server
ENTRYPOINT ["ghostmark"]
CMD ["serve", "--host", "0.0.0.0", "--port", "8080"]
