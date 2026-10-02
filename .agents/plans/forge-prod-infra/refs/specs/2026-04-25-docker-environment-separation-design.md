# Docker Environment Separation — Design Spec

**Date**: 2026-04-25
**Status**: Approved
**Parent**: forge-xk0z (Infrastructure initiative)

## Problem

Forge has a single Docker Compose stack for development. There is no way to:
- Run isolated test instances with synthetic data (protecting production data)
- Run multiple Forge stacks simultaneously (e.g., testing TS fix while doing Rust dev)
- Simulate production builds locally (compiled assets, no bind mounts)

## Design

### Dockerfile — Three Targets (Single File)

```
base (oven/bun:1.3-debian + sqlite3)
  └── deps (bun install --frozen-lockfile)
        ├── dev (bind mount, hot-reload, source code)
        │     └── test (extends dev + seed SQL + test runner entrypoint)
        ├── build (bun build core + svelte webui)
        │     └── prod (compiled assets only, no source, no dev deps, non-root)
```

- **dev**: Current behavior unchanged. Bind mounts, hot-reload, `bun run dev`.
- **test**: Extends dev. COPYs seed SQL. Entrypoint: migrate → seed → test → exit.
- **build**: Intermediate stage. Compiles TS core + Svelte WebUI to static output.
- **prod**: From base (not dev). Copies compiled output from build. Serves static Svelte via the API server. No source code, no dev deps, no bind mounts. Non-root `bun` user.

### Compose Profiles & Port Allocation

Single `compose.yml` with three profiles. Container names prefixed per environment to allow simultaneous stacks.

| Service     | dev            | test           | prod            |
|-------------|----------------|----------------|-----------------|
| Traefik     | 80 / 8080      | 9080 / 9081    | 7080 / 7081     |
| API (core)  | 3000           | 9000           | 7000            |
| WebUI       | 5173 (vite)    | 9173 (vite)    | — (embedded)    |
| MCP         | 5174           | 9174           | 7174            |
| MCP Inspect | 6274           | —              | —               |
| HelixDB     | 6969 (opt)     | —              | 6969 (opt)      |
| Data        | ./data-docker/ | ./data-test/   | forge-prod vol  |
| Source      | bind mount     | bind mount     | baked in        |
| Hostnames   | forge.localhost| test.localhost | prod.localhost  |
| Prefix      | forge-         | test-          | prod-           |

Key decisions:
- Test stack has no MCP inspector (ephemeral, no debugging needed)
- Test stack has no HelixDB (seeds run against SQLite only; HelixDB integration testing uses `--profile helix` on dev)
- Prod embeds WebUI into the API container (no separate Vite container)
- Environment selected via `FORGE_ENV` variable, justfile recipes set it

### Test Seed Script

**File**: `packages/core/src/db/seeds/test-seed.sql`

Programmatic SQL with deterministic IDs (`test-{entity}-{NNN}`):

```
2 organizations (1 with HQ location+address, 1 remote-only)
3 source_roles (hybrid, remote, contract across both orgs)
3 sources (1 per role)
6 bullets (2 per source, with technologies)
4 perspectives (mixed archetypes/domains/framings)
1 profile (fake name/email/phone + address)
3 archetypes (use migration-seeded: security-engineer, infrastructure, agentic-ai)
1 resume with 3 sections (experience, skills, education)
  - experience entries linked to perspectives
  - skills with resume_skills junctions
  - education with source_education
2 certifications (with certification_skills)
1 credential (clearance type)
3 answer_bank entries
1 job_description (with parsed_sections)
All junction tables: bullet_sources, bullet_skills, archetype_domains
```

Deterministic IDs allow tests to reference entities by known values without querying.

**Maintenance strategy**: Seed lives alongside migrations. Schema changes that affect seeded tables must update the seed in the same PR — CI enforces this because the test profile runs the seed, and a broken seed fails the test suite.

### Test Lifecycle

`just docker test` (CI-friendly, ephemeral):

1. Build test target image
2. `docker compose --profile test up -d` (traefik + core + webui + mcp)
3. Wait for core health check
4. Container entrypoint: migrate → seed → `bun test` → exit
5. `docker compose --profile test down -v` (wipe data-test/)
6. Just recipe exits with container's exit code

### Just Recipes

```bash
# Existing (unchanged)
just docker dev              # Dev stack, hot-reload, data-docker/
just docker down             # Stop all stacks
just docker logs [svc]       # Follow logs
just docker shell            # Interactive container

# New
just docker test             # Ephemeral: build → seed → test → teardown
just docker prod             # Production simulation, compiled build
just docker build [env]      # Build images for env (dev/test/prod)
just docker reset [env]      # Wipe data for specific env

# Convenience
just docker test-seed        # Start test stack with seed, leave running
just docker test-shell       # Shell into running test container
```

`just docker run` remains as an alias for `just docker dev` (backward compat).

## Downstream Work (Separate Beads)

Ordered by dependency:

1. Observability (OTEL) — immediately after compose profiles land (existing forge-mar5, forge-24g9)
2. Production Dockerfile validation — after compose profiles
3. Local container registry — after prod Dockerfile
4. GHCR publishing — after local registry
5. CI Docker builds — after GHCR
6. K8s / Helm manifests — after GHCR
7. Container secrets management — after GHCR
8. Docker Hub publishing — after GHCR
