FROM rust:1.78 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rust-core /usr/local/bin/rust-core
COPY --from=builder /app/database/migrations /migrations
COPY --from=builder /app/selector-api /selector-api
EXPOSE 8080
CMD ["rust-core"]
