# Build stage
FROM rust:1.83-slim-bullseye AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build the application
RUN cargo build --release -p starfish-backend

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/starfish-backend /app/

# Copy the frontend
COPY --from=builder /app/starfish-frontend /app/starfish-frontend

# Expose port
EXPOSE 8080

# Run the application
CMD ["/app/starfish-backend"]
