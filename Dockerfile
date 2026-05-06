# syntax=docker/dockerfile:1.7

FROM oven/bun:1-alpine AS deps
WORKDIR /app
COPY package.json bun.lock* bun.lockb* ./
RUN bun install --frozen-lockfile

FROM oven/bun:1-alpine AS build
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
ARG GITHUB_TOKEN
ENV GITHUB_TOKEN=$GITHUB_TOKEN
RUN bun run build

FROM oven/bun:1-alpine AS runtime
WORKDIR /app
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=4321
COPY --from=build /app/dist ./dist
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/package.json ./package.json
RUN mkdir -p /app/data
VOLUME ["/app/data"]
EXPOSE 4321
HEALTHCHECK --interval=10s --timeout=5s --start-period=1s --retries=5 \
  CMD wget -qO- http://127.0.0.1:${PORT}/api/health || exit 1
CMD ["bun", "./dist/server/entry.mjs"]
