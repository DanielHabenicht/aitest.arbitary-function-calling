# Build stage
FROM rust:1.83 as builder

WORKDIR /app

# Copy manifest files (include Cargo.lock for reproducible builds)
COPY Cargo.toml ./

COPY src ./src

# Build the real application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/js-execution-service .

# Expose port
EXPOSE 3000

ENV PORT=3000

# Run the application
CMD ["./js-execution-service"]
