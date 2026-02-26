# Multi-stage Dockerfile for herakles-node-exporter
# Optimized for minimal runtime image size with musl

# Build stage - uses pre-built musl binaries from CI
FROM alpine:3.19 AS runtime

# Install minimal runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    && rm -rf /var/cache/apk/*

# Create non-root user for security
RUN addgroup -g 1000 herakles && \
    adduser -D -u 1000 -G herakles herakles

# Copy the pre-built binary (injected by CI pipeline)
# The binary is built with musl and statically linked
ARG TARGETPLATFORM
COPY --chmod=755 herakles-node-exporter /usr/local/bin/herakles-node-exporter

# Set up /proc access (read-only mount point for container runtime)
# The actual /proc mount is done at container runtime via -v /proc:/host/proc:ro

# Use non-root user
USER herakles

# Expose the default Prometheus metrics port
EXPOSE 9215

# Health check using busybox wget (included in Alpine)
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget -q -O /dev/null http://localhost:9215/health || exit 1

# Set entrypoint to the exporter binary
ENTRYPOINT ["/usr/local/bin/herakles-node-exporter"]

# Default command (can be overridden)
CMD []
