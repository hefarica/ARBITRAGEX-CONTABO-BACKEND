# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependencies
COPY Cargo.toml Cargo.lock ./

# Build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source
COPY src ./src
COPY migrations ./migrations

# Build application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary and migrations
COPY --from=builder /app/target/release/selector-api /app/selector-api
COPY --from=builder /app/migrations /app/migrations

# Create non-root user
RUN useradd -m -u 1000 api && chown -R api:api /app
USER api

# Environment variables
ENV RUST_LOG=selector_api=debug,axum=info
ENV PORT=8080
ENV DATABASE_URL=postgres://arbitragex:password@postgres-db:5432/arbitragex
ENV REDIS_URL=redis://redis-cache:6379

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run
CMD ["/app/selector-api"]



