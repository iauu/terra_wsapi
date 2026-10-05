FROM rust:1.93-slim as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.93-slim as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked,id=reg-${TARGETARCH} \
    --mount=type=cache,target=/app/target,sharing=locked,id=cache-${TARGETARCH} \
    cargo chef cook --release --recipe-path recipe.json

FROM rust:1.93-slim as builder
WORKDIR /app
COPY . .
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked,id=reg-${TARGETARCH} \
    --mount=type=cache,target=/app/target,sharing=locked,id=cache-${TARGETARCH} \
    cargo build --release
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked,id=reg-${TARGETARCH} \
    --mount=type=cache,target=/app/target,sharing=locked,id=cache-${TARGETARCH} \
    cp /app/target/release/terra_wsapi /app/terra_wsapi

FROM debian:sid-slim
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates
COPY --from=builder /app/terra_wsapi /app/terra_wsapi
CMD ["./terra_wsapi"]