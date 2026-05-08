# syntax=docker/dockerfile:1.7

FROM oven/bun:1.3.5-alpine AS bun-bin

FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static ca-certificates
COPY --from=bun-bin /usr/local/bin/bun /usr/local/bin/bun
WORKDIR /app

# Copy dependency metadata first so Docker/BuildKit can reuse dependency caches
# when only application source files change.
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/storage/Cargo.toml crates/storage/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY frontend/package.json frontend/bun.lock ./frontend/

RUN --mount=type=cache,target=/root/.bun/install/cache \
  cd frontend && bun install --frozen-lockfile

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/app/target \
  cargo build --release -p ideas-server && \
  cp /app/target/release/ideas /usr/local/bin/ideas

FROM alpine:3.20 AS runtime
WORKDIR /app
RUN apk add --no-cache ca-certificates wget && mkdir -p /app/data
ENV HOST=0.0.0.0
ENV PORT=4321
ENV DB_PATH=/app/data/ideas.db
COPY --from=build /usr/local/bin/ideas /usr/local/bin/ideas
VOLUME ["/app/data"]
EXPOSE 4321
HEALTHCHECK --interval=10s --timeout=5s --start-period=1s --retries=5 \
  CMD wget -qO- http://127.0.0.1:${PORT}/api/health || exit 1
CMD ["ideas", "serve"]
