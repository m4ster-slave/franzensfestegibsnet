FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/franzensfestegibsnet /usr/local/bin/franzensfestegibsnet
COPY templates ./templates
COPY public ./public
COPY migrations ./migrations
COPY Rocket.toml ./Rocket.toml

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8080
ENV UPLOAD_DIR=/app/uploads

EXPOSE 8080

CMD ["franzensfestegibsnet"]
