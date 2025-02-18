FROM rust:1.84-bullseye AS builder

RUN apt-get update && apt-get install -y \
    libssl-dev \
    cmake \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/bot
COPY Cargo.toml Cargo.lock ./

# RUN cargo fetch

COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/bot

COPY --from=builder /usr/src/bot/target/release/valeriyya .

CMD ["./valeriyya"]