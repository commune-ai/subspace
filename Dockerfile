# **NOTE**: This docker file expects to be run in a directory outside of subspace.
# It also expects two build arguments, the bittensor snapshot directory, and the bittensor
# snapshot file name.

# This runs typically via the following command:
# $ docker build -t subspace . --platform linux/x86_64 --build-arg SNAPSHOT_DIR="DIR_NAME" --build-arg SNAPSHOT_FILE="FILENAME.TAR.GZ"  -f subspace/Dockerfile


FROM ubuntu:22.10
SHELL ["/bin/bash", "-c"]

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG SNAPSHOT_DIR
ARG SNAPSHOT_FILE

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

WORKDIR /app
# install rust  
COPY ./scripts /app/scripts
RUN chmod +x /app/scripts/*
RUN /app/scripts/install_rust_env.sh

COPY ./pallets /app/pallets
COPY ./runtime /app/node
COPY ./node /app/subspace
COPY ./target /app/target
COPY ./.env /app/.env
COPY ./Cargo.toml /app/Cargo.toml

RUN cargo build --release




