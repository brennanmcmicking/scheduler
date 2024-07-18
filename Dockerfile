FROM rust:latest AS build

RUN USER=root cargo new --bin scheduler
WORKDIR /scheduler

COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm ./target/release/deps/scheduler*
RUN cargo build --release

FROM debian:stable-slim

COPY --from=build /scheduler/target/release/scheduler .

CMD ["./scheduler"]
