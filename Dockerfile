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

LABEL ai.opentensor.image.authors="operations@opentensor.ai" \
        ai.opentensor.image.vendor="Opentensor Foundation" \
        ai.opentensor.image.title="opentensor/subspace" \
        ai.opentensor.image.description="Opentensor subspace Blockchain" \
        ai.opentensor.image.revision="${VCS_REF}" \
        ai.opentensor.image.created="${BUILD_DATE}" \
        ai.opentensor.image.documentation="https://opentensor.gitbook.io/bittensor/"

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

RUN mkdir -p subspace/scripts
RUN mkdir -p subspace/specs

COPY subspace/scripts/init.sh subspace/scripts/init.sh
COPY subspace/specs/nakamotoChainSpecRaw.json subspace/specs/nakamotoSpecRaw.json

RUN subspace/scripts/init.sh

COPY ./subspace/target/release/node-subspace /usr/local/bin

RUN /usr/local/bin/node-subspace --version

COPY ${SNAPSHOT_DIR}/${SNAPSHOT_FILE}.tar.gz /subspace

RUN mkdir -p /root/.local/share/node-subspace/chains/nakamoto_mainnet/db/full
RUN tar -zxvf /subspace/${SNAPSHOT_FILE}.tar.gz -C  /root/.local/share/node-subspace/chains/nakamoto_mainnet/db/full

RUN apt remove -y curl
RUN rm /subspace/${SNAPSHOT_FILE}.tar.gz

EXPOSE 30333 9933 9944
