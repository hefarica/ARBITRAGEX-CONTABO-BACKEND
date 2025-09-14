# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy dependencies
COPY Cargo.toml Cargo.lock ./

# Build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source
COPY src ./src

# Build application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies and Foundry
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Foundry (includes Anvil)
RUN curl -L https://foundry.paradigm.xyz | bash
RUN /root/.foundry/bin/foundryup

# Copy Anvil to PATH
RUN cp /root/.foundry/bin/anvil /usr/local/bin/

# Copy binary
COPY --from=builder /app/target/release/sim-ctl /app/sim-ctl

# Create non-root user
RUN useradd -m -u 1000 simulator && chown -R simulator:simulator /app
USER simulator

# Environment variables
ENV RUST_LOG=sim_ctl=debug,ethers=info
ENV PORT=8081
ENV ANVIL_PATH=/usr/local/bin/anvil
ENV FORK_URL=http://geth-node:8545

# Expose port
EXPOSE 8081

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8081/health || exit 1

# Run
CMD ["/app/sim-ctl"]



