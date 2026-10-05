# Migration: MVP 2.0 — Browser-First Architecture

> Status: Planning
> Goal: CF-deployed minimal browser-first example ASAP, then iterate
> Self-destruct: Delete this file when migration is complete

## What shipped (minimal Rust version, M0-M7)

The browser-first app now exists, built differently from the plan below. Read this section first;
the phases that follow are the original plan and are kept for the parts that did not ship.

**Shipped.** The web UI runs with no server by default. The Rust API (`forge-api`, an axum `Router`
over `forge-sdk`) runs in a dedicated Worker, storing a SQLite database in the browser's OPFS
through `rusqlite` on `sqlite-wasm-rs`, not wa-sqlite
([ADR 0001](../dev/adrs/rust-wasm/0001-rusqlite-and-axum-in-a-browser-worker.md)).

- **Owner tab.** One tab owns the database; a second tab waits and takes over when the first
  closes. Export and import of the database file are in Settings → Storage
  ([ADR 0002](../dev/adrs/rust-wasm/0002-browser-runtime-ownership-and-transport.md)).
- **PDFs.** Typst, compiled in the browser by a separate 25 MB module that is only fetched on the
  first PDF ([ADR 0003](../dev/adrs/rust-wasm/0003-typst-pdf-in-both-hosts.md)).
- **Derivation.** There is no server-side model call. Claude in Chrome drives the open tab through
  `window.forge` (the UI's own client), and the UI refetches on a `forge:changed` event. The skill is
  `.claude/skills/forge-in-chrome/SKILL.md`.
- **Default mode is the browser runtime.** `VITE_FORGE_MODE=api` selects the HTTP API (the TypeScript
  server, or `forge-server`) instead; `just dev` and the Docker stacks do. The TypeScript server is not
  deleted. `just app` builds the wasm bundles and starts the browser-first UI.
- **Acceptance.** `packages/webui/e2e/wasm/core-loop.spec.ts` drives profile, organization, source,
  bullet, approve, perspective, approve, resume from a template and PDF in Chromium, with no server. It
  runs in CI (`core loop in the browser`).

**Not part of the minimal version.** wa-sqlite, the CDN snapshot, D1 and sync, HelixDB, and the
extension sync service (phases 1 and 3 to 6 below). Only the dual-mode application (phase 2) was
done, and as a build-time default rather than a runtime switch. The static app itself is deployed
to Cloudflare Pages; see `.github/AGENTS.md` (`app-build.yml`).

**Known gaps.** Chrome is the only browser verified. A resume's `latex_override` is not compiled in the
browser (the PDF is generated from the resume content, with a notice). Several list pages other than
sources and bullets/perspectives do not yet refetch on `forge:changed`. No real `forge.db` has been
imported by the project's automated checks; that is a manual step (Settings → Storage → Import).

## Overview

Migrate from the current server-first architecture (Hono API as primary, browser as thin client) to browser-first (wa-sqlite as primary, server as optional SaaS enhancement).

The goal is a working CF deployment as fast as possible, then iterating on features.

## Current State (MVP 1.0)

```
Browser (Svelte) ──HTTP──→ Hono API ──→ SQLite (server)
Extension ──HTTP──→ Hono API
MCP Server ──HTTP──→ Hono API
```

- All data lives on server
- Browser is a thin client making API calls
- No offline capability
- Self-hosted only (no cloud deployment)

## Target State (MVP 2.0)

```
Browser (WASM) ──→ wa-sqlite (OPFS, local)
                ──→ CDN (global snapshot)
                ──→ Server API (SaaS sync only)
Extension ──→ wa-sqlite (same-origin, default)
           ──→ Hono API (configurable override)
```

## Phases

### Phase 1: ELM BrowserStore Adapter

**What:** Create a `WaSqliteAdapter` that implements the same ELM interface as `SqliteAdapter` but targets browser wa-sqlite.

**Changes:**
- New package or module: `packages/core/src/adapters/wa-sqlite/`
- Same repository interfaces, different storage backend
- Migration runner that works in browser (apply migrations to wa-sqlite)
- All existing SQL queries must work in wa-sqlite (verify compat)

**Docs to update:** `models/runtime.md` — add BrowserStore adapter details

**Done when:** Can run the full Forge data layer in a browser tab with wa-sqlite, no server.

### Phase 2: Dual-Mode Application

**What:** Application logic works with either BrowserStore or RemoteStore, selected at startup.

**Changes:**
- ELM initialization accepts a store type config
- OSS mode: `BrowserStore` (wa-sqlite, OPFS)
- SaaS mode: `BrowserStore` + `RemoteStore` (browser-local for reads, server for sync)
- Self-hosted mode: `RemoteStore` only (existing behavior, Hono API)
- Svelte app detects mode and initializes accordingly

**Docs to update:** `models/deployment.md` — dual-mode configuration

**Done when:** Same app binary works in all three modes.

### Phase 3: CF Deployment (MVP 2.0 ship target)

**What:** Minimal CF deployment — static app on R2/Pages, global snapshot on R2.

**Changes:**
- Build pipeline: Svelte app → WASM bundle → CF Pages
- Global skill graph snapshot: build + upload to R2
- Embedding model files: cache on R2 or reference HuggingFace CDN
- No SaaS features yet — pure OSS browser-local experience
- Domain setup (forge.arusty.dev or similar)

**Docs to update:** `models/deployment.md` — CF deployment specifics

**Done when:** Anyone can visit the URL and use Forge with browser-local storage.

### Phase 4: D1 + Sync (SaaS foundation)

**What:** Authenticated users get D1-backed persistence with CRDT sync.

**Changes:**
- D1 database provisioned per-tenant namespace
- CF Workers auth layer (CF Access / JWT)
- Sync endpoint on Hono API (CF Worker)
- CRDT operation log in browser wa-sqlite
- Push/pull sync protocol implementation

**Docs to update:** `models/sync.md` — implementation details, `seams.md` — Seam 7 details

**Done when:** SaaS user can use Forge on two devices and data syncs.

### Phase 5: HelixDB + Premium Features

**What:** Connect HelixDB for advanced graph queries, market intelligence, LLM curation.

**Changes:**
- HelixDB deployment on Vercel behind CF ZeroTrust
- Global skill graph curation pipeline (server-side)
- Market stats aggregation from federated contributions
- Premium API endpoints (analytics, 3rd-party data)

**Docs to update:** `graphs/computed.md` — market stats pipeline, `models/deployment.md` — HelixDB details

**Done when:** SaaS users get market intelligence and advanced graph features.

### Phase 6: Extension Sync Service

**What:** Extension syncs with browser-local DB by default, configurable for self-hosted.

**Changes:**
- Extension reads/writes same-origin wa-sqlite (no API calls needed)
- Configuration option for self-hosted Hono API override
- Sync Service mediates extension ↔ SaaS (browser is the intermediary)

**Docs to update:** `seams.md` — Seam 8 details

**Done when:** Extension works in both OSS and SaaS modes.

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| wa-sqlite OPFS browser compat | Blocks Phase 1 | Test in Chrome, Firefox, Safari early. Fallback: IndexedDB via sql.js |
| CRDT complexity | Delays Phase 4 | Start with LWW-only (simplest CRDT), add OR-Set for collections later |
| wa-sqlite query compat with server SQLite | Blocks Phase 1 | Run existing test suite against wa-sqlite adapter early |
| CDN snapshot size growth | Degrades OSS UX | Delta sync (Phase 5+), lazy loading, compression |
| D1 cold start latency | SaaS UX issue | Turso session cache layer |

## Cleanup (post-migration)

When all phases are complete:
- [ ] Delete this file
- [ ] Update `models/runtime.md` to remove "migration in progress" notes
- [ ] Update `models/deployment.md` to mark as "current" not "target"
- [ ] Archive any temporary compatibility shims
- [ ] Update CLAUDE.md to reflect browser-first as default
