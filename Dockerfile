FROM rust:latest as buildenv

WORKDIR /build
COPY . .
COPY .sqlx ./.sqlx/
RUN cargo build -p sayless --release

FROM ubuntu:latest

WORKDIR /sayless
COPY --from=buildenv /build/target/release/sayless /sayless/sayless
COPY config.toml /sayless/config.toml
COPY db /sayless/db/
COPY migrations /sayless/migrations/
ENTRYPOINT ["./sayless"]
