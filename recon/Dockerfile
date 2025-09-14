# Multi-stage build for recon service
FROM rust:1.75-slim as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    cron \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies including cron
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    cron \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder stage
COPY --from=builder /app/target/release/recon /usr/local/bin/recon

# Copy migrations
COPY --from=builder /app/migrations ./migrations

# Create non-root user
RUN useradd -r -s /bin/false recon

# Set up cron for periodic tasks
COPY crontab /etc/cron.d/recon-cron
RUN chmod 0644 /etc/cron.d/recon-cron
RUN crontab /etc/cron.d/recon-cron

# Create cron log file
RUN touch /var/log/cron.log

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=postgresql://postgres:password@postgres:5432/arbitragex
ENV REDIS_URL=redis://redis:6379
ENV D1_DATABASE_URL=d1://arbitragex-edge
ENV SYNC_ENABLED=true
ENV SYNC_INTERVAL_MINUTES=30
ENV SYNC_BATCH_SIZE=1000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep recon || exit 1

# Switch to non-root user
USER recon

# Expose port for potential HTTP API
EXPOSE 8084

# Create startup script
COPY --chown=recon:recon <<EOF /app/start.sh
#!/bin/bash
# Start cron in background
sudo service cron start
# Start the main application
exec recon
EOF

RUN chmod +x /app/start.sh

# Run the application
CMD ["/app/start.sh"]
