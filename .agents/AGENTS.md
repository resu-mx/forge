# Agent context index

Read this at the start of every conversation and again after every compaction. It lists
every agent-context file in the repo and when to read it. Claude Code loads a directory's
`AGENTS.md` by itself when it reads a file in that directory. Other agents should open the
file named below before working there.

## Before you work in a directory

| Read | Before touching |
|---|---|
| `packages/AGENTS.md` | `packages/**`: the Bun/TypeScript workspace |
| `packages/webui/AGENTS.md` | `packages/webui/**`: Svelte pages, components, CSS (shared-component rules) |
| `packages/extension/AGENTS.md` | `packages/extension/**`: the Chrome + Firefox extension |
| `crates/AGENTS.md` | `crates/**`, `Cargo.toml`, `rust-toolchain.toml` |
| `docs/AGENTS.md` | `docs/**` |
| `.docker/AGENTS.md` | `.docker/**`, or any `just docker …` work |
| `.github/AGENTS.md` | `.github/**`: CI and release workflows |

## Before you do a task

| Read | Before you |
|---|---|
| `.agents/context/beads-taxonomy.md` | create or label a bead (`cat:` labels) |
| `.agents/plans/rust-mvp-roadmap.md` | work on the Rust port (milestones M0–M7 and their status) |
| `docs/src/dev/adrs/` | change something an ADR decided |
| `.claude/skills/forge-in-chrome/SKILL.md` | read or write Forge data through an open wasm-mode Forge tab |

## Plans: `.agents/plans/`

Working documents for work in progress: specs, phased plans, prompts, research. Once a
conclusion is durable it moves to `docs/` (see `docs/AGENTS.md`).

- Forge development (UI/UX, data models, features, bugs) goes in `.agents/plans/forge-resume-builder/`:
  - design specs: `refs/specs/YYYY-MM-DD-<topic>.md`
  - phased plans: `phase/<number>-<name>.md`
  - feature requests and bugs: `.feats/<category>/` (MCP: `.feats/mcp/`)
- Every other topic gets its own directory: `.agents/plans/<topic>/` (for example
  `forge-resume-browser-extension/`, `forge-prod-infra/`, `claude-code-telemetry/`). A
  single-document plan may sit at the top level, like `rust-mvp-roadmap.md`.
- Never put specs or plans in `docs/superpowers/`. That path is a superpowers-skill default
  and does not apply here.

## Claude Code only

| Path | What |
|---|---|
| `.claude/settings.json` | hooks: `bd prime` on SessionStart and PreCompact |
| `.claude/rules/` | path-scoped pointers to the `AGENTS.md` files above; no content of their own |
| `.claude/skills/` | Claude skills (`forge-in-chrome`) |
| `.claude/agents/dioxus.md` | Dioxus 0.7 reference prompt for the planned Dioxus UI (roadmap R4) |
| `.claude/serena/project.yml` | Serena language-server config |
| `.mcp.json` | MCP servers: `forge-mcp` (HTTP `:5174/mcp`, needs `just dev`), `chrome-devtools` |

Claude Code telemetry (mitmproxy → MLFlow, `just telemetry`) is designed in
`.agents/plans/claude-code-telemetry/`. Use MLFlow at `http://127.0.0.1:5000`, not
`localhost`: AirPlay holds :5000 on IPv6. The recipes call `scripts/*.py`, which is not in
the repo yet.

## Maintaining agent context

- Add a row above for every new directory `AGENTS.md`. Agents other than Claude only find
  files through this index.
- Each fact lives in exactly one file. Link instead of copying. The root `AGENTS.md` carries
  only what every task needs.
- Keep this index under about 80 lines; detail belongs in the file it points to.
- Create a directory under `.agents/` only when its first real document exists. No stubs.
- Never put secrets, credentials or transcripts in agent context.
- Human overview: `.agents/README.md`.
