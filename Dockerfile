FROM debian:12-slim AS builder

ARG AWS_ACCESS_KEY_ID
ARG AWS_SECRET_ACCESS_KEY
ARG SCCACHE_BUCKET
ARG SCCACHE_ENDPOINT
ENV SCCACHE_REGION=auto

WORKDIR /app
COPY . .

# Dependencies using during the build stage.
RUN apt update && apt install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    protobuf-compiler \
    libclang-dev \
    git

ENV PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Installs rust with a minimal footprint and adds the WASM chain. 
RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- -y --profile=minimal --default-toolchain=nightly-2024-02-01

RUN if [ -n "$SCCACHE_BUCKET" ]; then \
        curl https://github.com/mozilla/sccache/releases/download/v0.7.7/sccache-v0.7.7-x86_64-unknown-linux-musl.tar.gz \
            -Lo sccache-v0.7.7-x86_64-unknown-linux-musl.tar.gz && \
        tar -xzf sccache-v0.7.7-x86_64-unknown-linux-musl.tar.gz --strip-components=1 \
            sccache-v0.7.7-x86_64-unknown-linux-musl/sccache && \
        ./sccache --start-server && \
        export RUSTC_WRAPPER="/app/sccache"; \
    fi && \
    cargo build -p node-subspace --release --locked

RUN if [ -n "$SCCACHE_BUCKET" ]; then \
        ./sccache --show-stats; \
    fi

FROM debian:12-slim

RUN apt update && apt install -y zlib1g && \
    rm -rf /var/cache/apt/archives /var/lib/apt/lists/*

COPY --from=builder /app/target/release/node-subspace /usr/local/bin

WORKDIR /subspace

ENV RUST_BACKTRACE=1
CMD ["node-subspace"]
