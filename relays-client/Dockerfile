# Multi-stage build for relays-client service
FROM rust:1.75-slim as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder stage
COPY --from=builder /app/target/release/relays-client /usr/local/bin/relays-client

# Create non-root user
RUN useradd -r -s /bin/false relays-client

# Set environment variables
ENV RUST_LOG=info
ENV FLASHBOTS_RELAY_URL=https://relay.flashbots.net
ENV EDEN_RELAY_URL=https://api.edennetwork.io/v1
ENV FLASHBOTS_ENABLED=true
ENV EDEN_ENABLED=true
ENV FLASHBOTS_PRIORITY=1
ENV EDEN_PRIORITY=2

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep relays-client || exit 1

# Switch to non-root user
USER relays-client

# Expose port for potential HTTP API
EXPOSE 8083

# Run the application
CMD ["relays-client"]
