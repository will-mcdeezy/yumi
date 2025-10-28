# Build stage
FROM rust:latest AS builder

WORKDIR /usr/src/yumi

# Copy the entire project
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM ubuntu:22.04

# Install OpenSSL and ca-certificates
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/yumi/target/release/yumi /app/

# Expose the port your server runs on
EXPOSE 8080

# Run the binary
CMD ["./yumi"]