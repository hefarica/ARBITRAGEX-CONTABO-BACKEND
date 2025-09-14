# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Instalar dependencias del sistema
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar archivos de dependencias
COPY Cargo.toml Cargo.lock ./

# Crear src dummy para cachear dependencias
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copiar código fuente real
COPY src ./src

# Rebuild con el código real
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Instalar certificados CA y librerías runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copiar binario compilado
COPY --from=builder /app/target/release/searcher-rs /app/searcher-rs

# Crear usuario no-root
RUN useradd -m -u 1000 searcher && chown -R searcher:searcher /app
USER searcher

# Configurar variables de entorno
ENV RUST_LOG=searcher_rs=debug,web3=info
ENV RPC_URL=http://geth-node:8545
ENV WS_URL=ws://geth-node:8546
ENV REDIS_URL=redis://redis-cache:6379
ENV DATABASE_URL=postgres://arbitragex:password@postgres-db:5432/arbitragex

# Healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/app/searcher-rs", "--health-check"] || exit 1

# Ejecutar
CMD ["/app/searcher-rs"]

