FROM rust:latest AS build

RUN USER=root cargo new --bin scheduler
WORKDIR /scheduler

COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo build --release

FROM debian:stable-slim

COPY --from=build /scheduler/target/release/backend .
COPY ./assets ./assets

CMD ["./backend"]
