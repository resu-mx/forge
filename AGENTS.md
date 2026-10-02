# Forge

Resume builder and job-application toolkit: career data (sources, bullets, perspectives,
skills) in, targeted resumes out.

> **Forge is a SaaS product + OSS distribution**, not a solo/personal tool. The current single-user local deployment is the MVP/dogfooding phase. All architectural decisions must account for multi-tenancy, auth, feature flags, observability, and data isolation. Do not make decisions that paint the project into a single-user corner.

## Start here — every conversation, and again after every compaction

Read `.agents/AGENTS.md` before doing anything else. It is the index of all agent context in
this repo and names the `AGENTS.md` to read before you touch each part of the tree. After a
compaction, read it again rather than working from a summary of it. (Claude Code also gets it
through the import below.)

@.agents/AGENTS.md

## Layout

| Path | What | Context |
|---|---|---|
| `packages/` | Bun/TypeScript workspace — the product as it runs today | `packages/AGENTS.md` |
| `crates/` | Rust workspace — the browser-first TS → Rust port | `crates/AGENTS.md` |
| `docs/` | Durable reference: architecture, ADRs, migrations | `docs/AGENTS.md` |
| `.docker/` | Compose stacks (dev / test / prod) behind Traefik | `.docker/AGENTS.md` |
| `.github/` | CI and release workflows | `.github/AGENTS.md` |
| `data/` | Local SQLite database (`data/forge.db`, gitignored) and the `just data` module | — |
| `.agents/` | Vendor-neutral agent context: the index, shared context, plans | `.agents/AGENTS.md` |
| `.claude/` | Claude Code-only config: rules, skills, subagents, hooks | `.agents/AGENTS.md` |

## Develop

**Use Docker by default.** The containerized stack gives isolated data, a consistent
environment and Traefik routing. Use the host-local recipes only when debugging host-level
issues.

```bash
just docker dev     # dev stack, hot reload, http://forge.localhost/ui/
just docker test    # ephemeral: build → seed → test → teardown
just docker down    # stop every profile

just setup          # host: bun install, .env from .env.example
just dev            # host: TS API :3000 + MCP :5174 + web UI :5173
just test           # all TS package tests + cargo test (or: just test core|sdk|mcp|cli|rust)
just data migrate   # SQLite migrations
just server         # Rust API server (forge-server) on :3000
```

## Invariants

- **Two runtimes, one contract.** The TypeScript API (`packages/core/src/routes`) and the Rust
  API (`crates/forge-api`) serve the same JSON contract, and the TS route tests are the
  contract tests (`just parity-tier0`, also in CI). A route change lands in both or the parity
  job fails.
- **Plans and specs go in `.agents/plans/`**, never `docs/superpowers/`. Layout in the index.

## Shell

`cp`, `mv` and `rm` may be aliased to `-i` and hang the agent waiting for y/n. Always pass
non-interactive flags: `cp -f`, `mv -f`, `rm -f`, `rm -rf`, `cp -rf`; `ssh`/`scp -o
BatchMode=yes`; `apt-get -y`; `HOMEBREW_NO_AUTO_UPDATE=1 brew …`.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
