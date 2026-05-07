# syntax=docker/dockerfile:1.7

FROM oven/bun:1.3.5-alpine AS bun-bin

FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static ca-certificates
COPY --from=bun-bin /usr/local/bin/bun /usr/local/bin/bun
WORKDIR /app
COPY . .
RUN cd frontend && bun install --frozen-lockfile
RUN cargo build --release -p ideas-server

FROM alpine:3.20 AS runtime
WORKDIR /app
RUN apk add --no-cache ca-certificates wget && mkdir -p /app/data
ENV HOST=0.0.0.0
ENV PORT=4321
ENV DB_PATH=/app/data/ideas.db
COPY --from=build /app/target/release/ideas /usr/local/bin/ideas
VOLUME ["/app/data"]
EXPOSE 4321
HEALTHCHECK --interval=10s --timeout=5s --start-period=1s --retries=5 \
  CMD wget -qO- http://127.0.0.1:${PORT}/api/health || exit 1
CMD ["ideas", "serve"]
