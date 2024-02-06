FROM rust:1.75-bookworm as build-env

RUN apt-get update
RUN apt-get install -y clang protobuf-compiler
WORKDIR /app
COPY . /app
RUN cargo build --release


# FROM gcr.io/distroless/cc
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y zlib1g && \
    rm -rf /var/cache/apt/archives /var/lib/apt/lists/*

COPY --from=build-env /app/target/release/node-subspace /usr/local/bin

WORKDIR /subspace

RUN mkdir -p ./snapshots
RUN ln -s -T /node-data/snapshots/main.json ./snapshots/main.json 

ENTRYPOINT ["node-subspace"]
