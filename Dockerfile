# CleanServe - Production Dockerfile (Alpine)
# Multi-stage build for minimal image size

# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.77-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Set up working directory
WORKDIR /build

# Copy source code
COPY . .

# Build release binary (static)
RUN cargo build --release --target x86_64-unknown-linux-musl

# ============================================
# Stage 2: Runtime
# ============================================
FROM alpine:3.19 AS runtime

# Labels
LABEL maintainer="CleanServe Team"
LABEL description="Zero-Burden PHP Runtime & Development Server"

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tzdata \
    openssl \
    && update-ca-certificates

# Create non-root user
RUN addgroup -g 1000 cleanserve && \
    adduser -u 1000 -G cleanserve -h /app -s /bin/sh -D cleanserve

# Create directories
RUN mkdir -p /app && \
    mkdir -p /cleanserve/bin && \
    mkdir -p /cleanserve/certs && \
    chown -R cleanserve:cleanserve /app /cleanserve

# Copy binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/cleanserve /usr/local/bin/

# Make executable
RUN chmod +x /usr/local/bin/cleanserve

# Set environment
ENV CLEANSERVE_HOME=/cleanserve
ENV PATH="/usr/local/bin:${PATH}"

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/healthz || exit 1

# Switch to non-root user
USER cleanserve

# Default command
WORKDIR /app
ENTRYPOINT ["/usr/local/bin/cleanserve"]
CMD ["up"]
