
ARG BASE_IMAGE=ubuntu:20.04

FROM $BASE_IMAGE as builder
SHELL ["/bin/bash", "-c"]

# This is being set so that no interactive components are allowed when updating.
ARG DEBIAN_FRONTEND=noninteractive

# show backtraces
ENV RUST_BACKTRACE 1

# Necessary libraries for Rust execution
RUN apt-get update && apt-get install -y curl build-essential protobuf-compiler clang git

# Install cargo and Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN mkdir -p /subspace
WORKDIR /subspace
COPY ./scripts ./scripts

# Update to nightly toolchain
RUN ./scripts/init.sh

# # Cargo build
# RUN cargo build --release --features runtime-benchmarks --locked
# EXPOSE 30333 9933 9944

# FROM $BASE_IMAGE
# COPY --from=builder /subspace/snapshot.json /
# COPY --from=builder /subspace/target/release/node-subspace /
# COPY --from=builder /subspace/raw_spec.json .

