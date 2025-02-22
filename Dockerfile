FROM rust:bookworm AS build

RUN USER=root cargo new --bin scheduler
WORKDIR /scheduler

COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

# RUN rustup component add clippy
# RUN cargo clippy --all-targets --all-features
RUN cargo build --release

RUN cargo run --bin scraper -- --oldest 202409

FROM debian:stable

RUN apt-get update
RUN apt-get install -y libssl3 ca-certificates

COPY --from=build /scheduler/target/release/backend .
COPY ./assets ./assets
COPY --from=build /scheduler/sections* ./

CMD ["./backend"]
