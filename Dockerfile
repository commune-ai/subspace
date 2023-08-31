# **NOTE**: This docker file expects to be run in a directory outside of subspace.
# It also expects two build arguments, the bittensor snapshot directory, and the bittensor
# snapshot file name.

# This runs typically via the following command:
# $ docker build -t subspace . --platform linux/x86_64 --build-arg SNAPSHOT_DIR="DIR_NAME" --build-arg SNAPSHOT_FILE="FILENAME.TAR.GZ"  -f subspace/Dockerfile


FROM ubuntu:22.10
SHELL ["/bin/bash", "-c"]

# This is being set so that no interactive components are allowed when updating.
ARG DEBIAN_FRONTEND=noninteractive
# show backtraces
ENV RUST_BACKTRACE 1

# Necessary libraries for Rust execution
RUN apt-get update && apt-get install -y curl build-essential protobuf-compiler clang git

WORKDIR /subspace

COPY ./scripts ./scripts
# Install cargo and Rust

RUN ./scripts/install_rust_env.sh

# Copy the source code
COPY . .
# Cargo build
RUN cargo build --release --locked



