# Build stage
FROM rust:1.83-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY libs libs
COPY services services

# Build the application
RUN cargo build --release --bin api

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 -U app

# Copy the binary from builder
COPY --from=builder /app/target/release/api /usr/local/bin/api

# Switch to non-root user
USER app

# Expose port
EXPOSE 3000

# Run the application
CMD ["api"]
