# Production Environment Specification

## MVP "Production" Model

For MVP, "production" means the user's local machine running the built application. There is no server deployment, cloud hosting, or multi-user scenario.

## Build

```bash
just build
# Builds all packages:
#   @forge/core — bundles to single file
#   @forge/sdk — compiles TypeScript
#   @forge/cli — bundles to executable
#   @forge/webui — vite build → static assets in dist/
```

## Running

```bash
# Single process serves both API and static WebUI
forge-server
# or: bun run packages/core/dist/index.js

# CLI is a separate binary
forge <command>
```

## Serving Strategy

In production mode, `@forge/core` serves:
- `/api/*` — HTTP API routes
- `/*` — static files from `packages/webui/dist/`

This means a single process, single port. The Vite dev server is not used in production.

## Database

Same SQLite file as development. No separate production database for MVP.
Backup strategy: `just dump` before upgrades.

## Future (Post-MVP)

- Tauri wraps the entire application into a native desktop binary
- Core becomes a Rust binary with embedded SQLite
- WebUI ships as Tauri's webview content
- CLI is a separate Rust binary using the SDK
