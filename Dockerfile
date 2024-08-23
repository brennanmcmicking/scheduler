FROM node:22-alpine3.19 as node
WORKDIR /scheduler

COPY ./package.json .
COPY ./package-lock.json .
COPY ./tailwind.config.js .

# need to copy all the rust code in since tailwind.config.js
# watches for .rs files
COPY ./src/ ./src/

RUN npm install
RUN npm run tw:build

FROM rust:latest AS build

RUN USER=root cargo new --bin scheduler
WORKDIR /scheduler

COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo build --release

FROM debian:stable-slim

COPY --from=build /scheduler/target/release/backend .
COPY ./assets ./assets
COPY --from=node /scheduler/assets/styles.css ./assets/styles.css

CMD ["./backend"]
