# `packages/`: Bun/TypeScript workspace

The product as it runs today. This is a Bun workspace (`packages/*` in the root
`package.json`), and every package is named `@forge/<dir>`.

## Packages

| Package | What | Stack | Checks (from the package dir) |
|---|---|---|---|
| `core` | HTTP API on `:3000` (`FORGE_PORT`): routes, services, storage, migrations | Bun + Hono, `bun:sqlite`, HelixDB adapter | `bun test`, `bun run migrate` |
| `sdk` | TypeScript API client (`ForgeClient`); the `fetch` it uses is injectable | none | `bun test`, `bun run typecheck` |
| `mcp` | MCP server over the SDK. stdio by default; HTTP on `:5174` with `FORGE_MCP_TRANSPORT=http` | `@modelcontextprotocol/sdk`, zod | `bun test` (integration tests skip if the API is down) |
| `webui` | web UI on `:5173` | SvelteKit 2, Svelte 5, Vite | see `webui/AGENTS.md` |
| `extension` | Chrome + Firefox MV3 extension | Svelte, Vite | `bun test`; see `extension/AGENTS.md` |
| `cli` | the `forge` CLI | citty | `bun test` |
| `runtime` | browser host for the Rust runtime: the Worker that owns the database, and a `fetch` that talks to it | none | `bun test`, `bun run typecheck` |
| `demo-data` | generated demo datasets: drives `forge-server` per persona, post-processes, verifies; see `demo-data/AGENTS.md` | `bun:sqlite` | `bun test` (integration needs `target/debug/forge-server`) |

`bun test` is Bun's built-in runner, so it works even where `package.json` has no `test`
script. From the repo root, `just test <core|sdk|mcp|cli>` runs one package's tests.

Dependency direction:

- `core`, `sdk` and `runtime` import no other workspace package.
- `mcp`, `webui`, `extension`, `cli` and `demo-data` use `@forge/sdk`.
- `mcp` and `extension` also import `@forge/core/src/parser`.
- `webui` uses `@forge/runtime` in wasm mode.
- Nothing imports `mcp`, `webui`, `extension`, `cli` or `demo-data`.

## Storage (`core`)

- The EntityLifecycleManager (ELM) abstracts all data access. Go through it, not around it.
- SQLite is the default adapter. For HelixDB, set
  `FORGE_STORAGE=helix HELIX_URL=http://localhost:6969` and run `just docker dev-helix`. HQL
  codegen is `just data gen` / `just data build`.
- Migrations live in `core/src/db/migrations/NNN_*.sql`, numbered sequentially. The Rust
  `forge-sdk` embeds the same files, so both runtimes run them.

## The API contract

`core/src/routes` defines the JSON API that `crates/forge-api` re-implements in Rust. The
route tests in `core/src/routes/__tests__` are the contract.

- `just parity-tier0` runs the Tier-0 files against the Rust server, and CI does too.
- Set `FORGE_TEST_SERVER_BIN` to point any route test at a server binary.
- If you change a route's shape, change both implementations.

The package READMEs (`mcp/README.md`, `extension/README.md`) predate the current code. Trust
the source over them.
