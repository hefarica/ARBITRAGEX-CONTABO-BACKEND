# Rust builder stage
FROM rust:1.72-slim-bookworm as builder

# Dependencias de sistema
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Crear usuario no-root
RUN useradd -m -u 1000 arbitragex

# Directorio de trabajo
WORKDIR /app

# Copiar solo archivos de dependencias primero para aprovechar la caché de Docker
COPY Cargo.toml Cargo.lock* ./

# Truco para cachear dependencias (crear un archivo dummy.rs y compilarlo)
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -f src/main.rs

# Copiar el código fuente
COPY . .

# Compilar con optimización
RUN cargo build --release

# Etapa final
FROM debian:bookworm-slim

# Instalar dependencias mínimas
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar el ejecutable compilado desde la etapa de builder
COPY --from=builder /app/target/release/arbitragex-mev-engine /usr/local/bin/

# Crear usuario no-root
RUN useradd -m -u 1000 arbitragex
USER arbitragex

# Directorio de configuración y datos
RUN mkdir -p /home/arbitragex/data /home/arbitragex/config

# Variables de entorno
ENV RUST_LOG=info
ENV CONFIG_FILE=/home/arbitragex/config/config.yaml

# Puerto para la API
EXPOSE 8000

# Comando para ejecutar
ENTRYPOINT ["arbitragex-mev-engine"]
