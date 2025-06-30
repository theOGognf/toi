ARG RELEASE=false

FROM rust:slim-bookworm AS chef

RUN apt-get update \
    && apt-get install -y \
        curl \
        gcc \
        libpq-dev \
        musl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-chef

WORKDIR /usr/app

FROM chef AS planner

ARG RELEASE

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

ARG RELEASE

COPY --from=planner /usr/app/build.sh build.sh
COPY --from=planner /usr/app/recipe.json recipe.json

RUN ./build.sh "cook" ${RELEASE}

COPY . .

RUN ./build.sh "bin" ${RELEASE}

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y \
        ca-certificates \
        libpq5 \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV RUST_LOG=info,tower_http=trace

WORKDIR /usr/app

COPY --from=builder /usr/app/toi_server/toi.json /usr/app/toi.json
COPY --from=builder /usr/local/bin/toi_server /usr/local/bin/toi_server

CMD ["toi_server"]
