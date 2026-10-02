# Development Environment Specification

## Prerequisites

- Bun >= 1.1 (runtime + package manager + test runner)
- Rust toolchain (rustup, cargo) — for stubs only, not required for TS development
- SQLite3 CLI — for manual database inspection
- Claude Code CLI (`claude`) — for AI module

## Environment Variables

File: `.env` (git-ignored), template: `.env.example`

```bash
FORGE_PORT=3000              # Core HTTP server port
FORGE_DB_PATH=./data/forge.db  # SQLite database file path
FORGE_CLAUDE_PATH=claude     # Path to claude binary (default: $PATH lookup)
FORGE_CLAUDE_TIMEOUT=60000   # AI derivation timeout in milliseconds
FORGE_LOG_LEVEL=info         # debug | info | warn | error
```

## Development Workflow

```bash
# Install dependencies
bun install

# Start development (core + webui)
just dev
# Equivalent to:
#   bun run --filter '@forge/core' dev   (port 3000)
#   bun run --filter '@forge/webui' dev  (port 5173, proxies API to 3000)

# Run tests
just test

# Run migrations
just migrate

# Database inspection
just shell   # opens sqlite3 on FORGE_DB_PATH
```

## Ports

| Service | Dev Port | Purpose |
|---|---|---|
| @forge/core | 3000 | HTTP API server |
| @forge/webui | 5173 | Vite dev server (proxies /api/* to core) |

## Database

- Location: `./data/forge.db` (default, configurable via FORGE_DB_PATH)
- WAL mode enabled for concurrent reads
- Foreign keys enforced on every connection
- Migrations run automatically on core server startup
- Backup: `just dump` creates timestamped SQL dump

## Workspace Package Resolution

Bun workspace protocol: consumer packages declare dependencies as `"@forge/sdk": "workspace:*"` in their package.json. Bun resolves these to the local package directory.
