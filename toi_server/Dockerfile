FROM rust:alpine AS chef

RUN apk update \
    && apk add --no-cache \
        gcc \
        musl-dev \
    && cargo install cargo-chef

WORKDIR /usr/app

FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release

FROM alpine:latest AS runtime

ENV RUST_LOG=info

WORKDIR /usr/app

COPY --from=builder /usr/app/target/release/toi_server ./toi_server

CMD ["./toi_server"]
