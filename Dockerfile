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

# install tools and dependencies
RUN apt-get update && \
        DEBIAN_FRONTEND=noninteractive apt-get install -y \
                ca-certificates \
                curl \
		clang && \
# apt cleanup
        apt-get autoremove -y && \
        apt-get clean && \
        find /var/lib/apt/lists/ -type f -not -name lock -delete;

# Install cargo and Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"
RUN . "$HOME/.cargo/env"
RUN echo "*** Initialized WASM build environment with Rust 1.68.1"

RUN rustup install nightly-2023-01-01
RUN rustup override set nightly-2023-01-01
RUN rustup target add wasm32-unknown-unknown


# # Use the "yes" command to automatically provide 'Y' as the answer
# RUN bash -c "yes | apt-get install libclang-dev"
# RUN bash -c "yes | apt-get install protobuf-compiler"


WORKDIR /subspace
RUN apt-get update
RUN yes | apt-get install libclang-dev
RUN yes | apt-get install protobuf-compiler
RUN apt-get install make

COPY . .
RUN cargo build --release




