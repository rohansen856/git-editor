# Stage 1: build
FROM rust:1.87-slim-bookworm AS builder

WORKDIR /usr/src/app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev make \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies first so source changes do not rebuild them.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && touch src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY . .
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# Stage 2: runtime
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 editor

COPY --from=builder /usr/src/app/target/release/git-editor /usr/local/bin/git-editor

# Run as an unprivileged user. Mount the repository at /workspace and pass
# --user "$(id -u):$(id -g)" so the repository owner matches (libgit2 refuses
# repositories owned by another user, like git's safe.directory check).
USER editor
WORKDIR /workspace
ENTRYPOINT ["git-editor"]
