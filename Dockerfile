FROM rust:latest AS build

RUN USER=root cargo new --bin scheduler
WORKDIR /scheduler

COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN rustup component add clippy
RUN cargo clippy --all-targets --all-features
RUN cargo build --release

FROM debian:stable-slim

COPY --from=build /scheduler/target/release/backend .
COPY ./assets ./assets

CMD ["./backend"]
