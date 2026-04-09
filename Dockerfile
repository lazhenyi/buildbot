# Buildbot Dispatcher - Rust CI System
#
# Multi-stage build:
# 1. build: compile Rust binary (heavy, build tools included)
# 2. runtime: minimal runtime image with just the binary + Docker CLI

# ─── Stage 1: Build ───────────────────────────────────────────────────────────

FROM --platform=$BUILDPLATFORM docker.io/library/rust:1.85-slim-bookworm AS build

ARG TARGETPLATFORM
ARG BUILDPLATFORM

# Install cross-compilation toolchains and build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        pkg-config \
        libssl-dev \
        gcc \
        # Build dependencies for sqlx
        libsqlite3-dev \
        libpq-dev \
        # For Docker-in-Docker support in runner
        docker.io \
        git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

# Build the project
RUN case "$TARGETPLATFORM" in \
        "linux/amd64")  RUST_TARGET=x86_64-unknown-linux-gnu ;; \
        "linux/arm64")  RUST_TARGET=aarch64-unknown-linux-gnu ;; \
        *)              RUST_TARGET=$(rustc -vV | sed -n 's/host: //p') ;; \
    esac && \
    cargo build --release --target "$RUST_TARGET" && \
    case "$TARGETPLATFORM" in \
        "linux/amd64")  cp "target/$RUST_TARGET/release/buildbot" /buildbot ;; \
        "linux/arm64")  cp "target/$RUST_TARGET/release/buildbot" /buildbot ;; \
        *)              cp "target/release/buildbot" /buildbot ;; \
    esac

# ─── Stage 2: Runtime (minimal) ──────────────────────────────────────────────

FROM docker.io/library/debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.authors="Buildbot Dispatcher"
LABEL org.opencontainers.image.description="Buildbot Dispatcher - GitHub Actions-style CI system"

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        dumb-init \
        git \
        # For Docker socket mount (runner executes containers)
        docker.io \
        # For PostgreSQL connectivity
        libpq5 \
        tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /bin/bash buildbot

WORKDIR /app

# Copy binary from build stage
COPY --from=build /buildbot /app/buildbot

# Copy sample config
COPY master.cfg.example /app/master.cfg.example

# Create data directory
RUN mkdir -p /app/data /app/repos && chown -R buildbot:buildbot /app

USER buildbot

EXPOSE 8010 9990

ENV BUILDBOT_BASEDIR=/app
ENV RUST_LOG=info

ENTRYPOINT ["/usr/bin/dumb-init", "--"]
CMD ["/app/buildbot", "master", "--basedir", "/app", "--config", "master.cfg"]
