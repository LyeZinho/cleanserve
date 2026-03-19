# Docker and Containerization

CleanServe provides a smooth experience within containerized environments. This guide covers how to set up Docker for both development and production.

## Production Setup

For production deployments, we recommend a **multi-stage build** using Alpine Linux for the smallest possible image footprint.

### Dockerfile Example

```dockerfile
# Stage 1: Build CleanServe
FROM rust:1.77-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /usr/src/cleanserve
COPY . .
RUN cargo build --release

# Stage 2: Runtime Image
FROM alpine:3.19
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN addgroup -S cleanserve && adduser -S cleanserve -G cleanserve
USER cleanserve:1000

# Set environment and home directory
ENV CLEANSERVE_HOME=/cleanserve
WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/cleanserve/target/release/cleanserve /usr/local/bin/cleanserve

# Expose server port
EXPOSE 8080

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=3s \
  CMD wget --quiet --tries=1 --spider http://localhost:8080/healthz || exit 1

ENTRYPOINT ["cleanserve", "up"]
```

## Development Setup

For local development with Docker, you can use a simpler `Dockerfile.dev` to mount your code and enable HMR.

### Dockerfile.dev

```dockerfile
FROM rust:1.77-alpine
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo install --path .
EXPOSE 8080 8081
CMD ["cleanserve", "up", "--port", "8080"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "8080:8080"
      - "8081:8081"
    volumes:
      - .:/app
    environment:
      - CLEANSERVE_HOME=/app/.cleanserve
```

## Building and Running

### Build the Production Image
```bash
docker build -t cleanserve .
```

### Run the Container
```bash
docker run -p 8080:8080 -v $(pwd):/app cleanserve
```

This mounts the current directory to `/app` in the container, allowing CleanServe to serve your project files directly.
