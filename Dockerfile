FROM rust:slim-bookworm AS chef

RUN apt-get update \
    && apt-get install -y \
        curl \
        gcc \
        libpq-dev \
        libssl-dev \
        musl-dev \
        openssl \
        pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-chef

WORKDIR /usr/app

FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release -p toi_server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y \
        libpq-dev \
        ca-certificates \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV RUST_LOG=info,tower_http=trace

WORKDIR /usr/app

COPY --from=builder /usr/app/target/release/toi_server ./toi_server

CMD ["./toi_server"]
