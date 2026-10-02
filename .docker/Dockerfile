# Forge Resume Builder — Multi-stage Docker image
#
# Targets:
#   dev  — bind-mount source, hot-reload (compose dev profile)
#   test — extends dev, seed data, runs tests then exits
#   build — intermediate: compiles TS core + MCP + Svelte WebUI
#   prod — compiled assets only, no source, no dev deps, non-root
#
# Build a specific target:
#   docker build --target prod -f .docker/Dockerfile .

FROM oven/bun:1.3-debian AS base
WORKDIR /app

# Install sqlite3 CLI for debugging/inspection inside the container
RUN apt-get update && apt-get install -y --no-install-recommends \
    sqlite3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# ───────────── deps stage ─────────────
FROM base AS deps

# Copy workspace manifests first for better layer caching
COPY package.json bun.lock ./
COPY packages/cli/package.json packages/cli/
COPY packages/core/package.json packages/core/
COPY packages/extension/package.json packages/extension/
COPY packages/mcp/package.json packages/mcp/
COPY packages/sdk/package.json packages/sdk/
COPY packages/webui/package.json packages/webui/

RUN bun install --frozen-lockfile

# ───────────── dev runtime ─────────────
# Used by compose.yml for the `dev` target — bind-mounts source code so
# bun's --watch picks up changes from the host.
FROM base AS dev

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/core/node_modules ./packages/core/node_modules
COPY --from=deps /app/packages/sdk/node_modules ./packages/sdk/node_modules
COPY --from=deps /app/packages/mcp/node_modules ./packages/mcp/node_modules
COPY --from=deps /app/packages/webui/node_modules ./packages/webui/node_modules

# Source will be bind-mounted at runtime; these COPYs are for standalone
# (non-compose) use.
COPY . .

# Create data directory owned by bun user
RUN mkdir -p /data && chown -R bun:bun /data /app
USER bun

ENV FORGE_DB_PATH=/data/forge.db
ENV FORGE_PORT=3000
ENV FORGE_MCP_TRANSPORT=http
ENV FORGE_MCP_PORT=5174
ENV FORGE_API_URL=http://core:3000

# Default: run core API. Override with CMD in compose.yml for other services.
CMD ["bun", "run", "--filter", "@forge/core", "dev"]

# ───────────── test runtime ─────────────
# Extends dev with seed data and a test-runner entrypoint.
# Entrypoint: migrate → seed → bun test → exit with test exit code.
FROM dev AS test

USER root
COPY packages/core/src/db/seeds/ /app/packages/core/src/db/seeds/
RUN chown -R bun:bun /app/packages/core/src/db/seeds/
USER bun

ENV FORGE_DB_PATH=/data/forge.db

ENTRYPOINT ["/bin/sh", "-c", "\
  bun run --filter @forge/core migrate && \
  for f in /app/packages/core/src/db/seeds/*.sql; do \
    [ -f \"$f\" ] && sqlite3 \"$FORGE_DB_PATH\" < \"$f\"; \
  done && \
  bun test \
"]

# ───────────── build stage ─────────────
# Compiles TS core, MCP server, and Svelte WebUI to static output.
# This is an intermediate stage — not used at runtime directly.
FROM deps AS build

COPY . .

RUN bun run --filter @forge/core build && \
    bun run --filter @forge/mcp build && \
    VITE_FORGE_MODE=api bun run --filter @forge/webui build

# ───────────── prod runtime ─────────────
# Minimal image with only compiled artifacts. No source code, no dev
# dependencies, no bind mounts. Runs as non-root bun user.
FROM base AS prod

# Copy compiled output from build stage
COPY --from=build /app/packages/core/dist/ ./packages/core/dist/
COPY --from=build /app/packages/mcp/dist/ ./packages/mcp/dist/
COPY --from=build /app/packages/webui/build/ ./packages/webui/build/

# Copy migrations where the compiled code expects them (import.meta.dir + 'db/migrations')
COPY --from=build /app/packages/core/src/db/migrations/ ./packages/core/dist/db/migrations/
COPY --from=build /app/package.json ./package.json
COPY --from=build /app/packages/core/package.json ./packages/core/package.json
COPY --from=build /app/packages/mcp/package.json ./packages/mcp/package.json

# Runtime-only dependencies (bun needs node_modules for resolution)
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/core/node_modules ./packages/core/node_modules
COPY --from=deps /app/packages/mcp/node_modules ./packages/mcp/node_modules

# Data directory, non-root user
RUN mkdir -p /data && chown -R bun:bun /data /app
USER bun

ENV FORGE_DB_PATH=/data/forge.db
ENV FORGE_PORT=3000
ENV FORGE_MCP_TRANSPORT=http
ENV FORGE_MCP_PORT=5174
ENV NODE_ENV=production

# Serve compiled API (which also serves static WebUI from packages/webui/build/)
CMD ["bun", "run", "packages/core/dist/index.js"]
