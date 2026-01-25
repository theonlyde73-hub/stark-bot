# Build stage
FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build the application
RUN cargo build --release -p stark-backend

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/stark-backend /app/

# Copy the frontend
COPY --from=builder /app/stark-frontend /app/stark-frontend

# Expose ports (HTTP + Gateway WebSocket)
EXPOSE 8080
EXPOSE 8081

# Run the application
CMD ["/app/stark-backend"]
