# `.docker/`: containerized environments

Docker is the default way to develop Forge (see the root `AGENTS.md`). This directory holds
`compose.yml` and a multi-stage `Dockerfile`, driven by the `just docker` module
(`.docker/justfile`). Every compose call passes `--project-directory <repo root>`, so paths in
`compose.yml` resolve from the repo root, not from this directory.

## Profiles

| Profile | Start with | Traefik (web / dashboard) | Host | Data |
|---|---|---|---|---|
| `dev` | `just docker dev` | 80 / 8080 | `forge.localhost` | `./data-docker/`, source bind-mounted, hot reload |
| `test` | `just docker test` (ephemeral), `just docker test-seed` (stays up) | 9080 / 9081 | `test.localhost` | `./data-test/`, seeded |
| `prod` | `just docker prod` | 7080 / 7081 | `prod.localhost` | `forge-prod-data` volume, compiled build |
| `helix` | `just docker dev-helix` (dev + HelixDB) | HelixDB on 6969 | none | `helix-data` volume |
| `tools` | `just docker shell` | none | none | `./data-docker/` |

Routes under each host:

- `/api/v1` goes to core.
- `/mcp` goes to the MCP server.
- `/mcp-dbg` goes to the MCP inspector (dev only).
- `/ui/` goes to the web UI. Prod has no web UI container; the API serves the built UI.

Other recipes:

- `build [env]`
- `down` (stops every profile)
- `logs [svc]`
- `test-shell`
- `reset [env]`, which wipes that environment's data directory or volume
- `run`, an old alias for `dev`

## Dockerfile targets

- `base` → `deps` → `dev` → `test`. `test` runs the core tests and exits.
- `deps` → `build` → `prod`. `prod` holds compiled assets only, with no source and no dev
  dependencies, and runs as non-root.

## Gotchas

- The stacks run only the TypeScript services, and their web UI is built with
  `VITE_FORGE_MODE=api` (the UI's default is the browser runtime). The Rust server
  (`just server`) and the wasm runtime (`just app`) build and run on the host.
- `just docker test` runs only `bun test` inside `test-core`. For other packages use
  `just test` on the host.
- Container data is separate from the host's `data/forge.db`. `just docker reset` never touches
  the host database.
- The dev services bind-mount all of `./packages` and shadow each package's `node_modules`
  with an anonymous volume. Only core, sdk, mcp, webui and cli are shadowed, so a new package
  a service depends on needs its own `node_modules` line in `compose.yml`.
