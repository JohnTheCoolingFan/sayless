FROM rust:latest as buildenv

WORKDIR /build
COPY . .
COPY .sqlx ./
RUN cargo build -p sayless --release

FROM alpine:latest
COPY --from=buildenv /build/target/release/sayless ./sayless
COPY config.toml ./
COPY db ./
COPY migrations ./
