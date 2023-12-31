FROM rust:latest as buildenv

WORKDIR /build
COPY . .
COPY .sqlx ./.sqlx/
RUN cargo build -p sayless --release

FROM debian:latest

WORKDIR /sayless
COPY --from=buildenv /build/target/release/sayless /sayless/sayless
COPY config.toml /sayless/config.toml
COPY migrations /sayless/migrations/
ENTRYPOINT ["/sayless/sayless"]
