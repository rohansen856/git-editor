# Stage 1: Build the application
FROM rust:1.81-slim AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
COPY Cargo.toml Cargo.lock ./

# Now copy the actual source code
COPY . .

# Build the application
RUN cargo build --release

# Stage 2: Create the runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/git-editor /app/git-editor

# Set up environment
ENV git-editor=/app/git-editor

# Set the entrypoint
ENTRYPOINT ["/app/git-editor"]