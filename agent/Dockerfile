FROM rust:1.79-slim-bullseye AS builder
WORKDIR /usr/src/robmon

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/robmon/target/release/robmon /usr/local/bin/robmon

ENV CONTROL_PLANE_URL=http://host.docker.internal:8080
ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/robmon"]
