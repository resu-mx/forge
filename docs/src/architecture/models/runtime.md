# Runtime Model

> Epic: forge-6r2y (Skill Graph Browser Runtime)
> Status: Design

> **As built (minimal Rust version, M0-M7).** What runs in the browser today is the data API, not the
> skill-graph runtime this page designs. `forge-wasm` hosts the `forge-api` router and `forge-sdk`
> stores in a dedicated Worker, over `rusqlite` and the OPFS `sahpool` VFS (so the "wa-sqlite
> Binding" section below did not happen). The page's UI talks to it through a `fetch` that posts to
> the Worker, and `window.forge` exposes the same client to an agent. PDFs come from a second,
> lazily loaded Typst module. Decisions and measurements:
> [ADR 0001](../../dev/adrs/rust-wasm/0001-rusqlite-and-axum-in-a-browser-worker.md),
> [0002](../../dev/adrs/rust-wasm/0002-browser-runtime-ownership-and-transport.md),
> [0003](../../dev/adrs/rust-wasm/0003-typst-pdf-in-both-hosts.md). Everything below this note,
> including snapshots, HNSW and the embedding model, is design that has not been built.

## Overview

Skill intelligence runs in two modes: server-side (build-time, full capability) and browser-side (runtime, snapshot-based). The architecture is a distillation pattern — LLM knowledge gets compressed into graph structures and embeddings that run cheaply on the client.

```
BUILD TIME (server, LLM, batch)          RUNTIME (browser, CPU, realtime)
┌─────────────────────────────┐          ┌──────────────────────────────┐
│ Curate skill taxonomy       │          │ Load graph snapshot          │
│ Generate embeddings         │  ──→     │ Load HNSW index              │
│ Propose relationships       │ snapshot │ Load small embedding model   │
│ Validate via human review   │          │ Extract → Match → Align      │
│ Build HNSW index            │          │                              │
└─────────────────────────────┘          └──────────────────────────────┘
```

## Crate: `forge-wasm`

The browser-side data layer is the `forge-wasm` crate (forge-f0gc). It compiles to `wasm32-unknown-unknown` and consumes [`forge-core`](#) types via wasm-bindgen. Built once and consumed by Svelte today, Tauri at R2, and Dioxus at R4 — the data layer doesn't change when the UI does.

**Layout**

```
crates/forge-wasm/
├── Cargo.toml          # cdylib + rlib, depends on forge-core (default-features = false)
└── src/lib.rs          # ForgeRuntime skeleton + wasm-bindgen exports
```

**Architectural rules**

- `forge-server` MUST NOT depend on `forge-wasm`. Enforced passively by omitting the crate from `[workspace.dependencies]` in the root `Cargo.toml` — adding it would have to be an explicit `path = "../forge-wasm"` declaration, which stands out in PR review.
- `forge-core` exposes its rusqlite-bound `ForgeError::Database` variant only when its `rusqlite` feature is on. Native consumers (forge-sdk, forge-server, forge-cli, forge-mcp) opt in via `features = ["rusqlite"]`. `forge-wasm` opts out, so the C SQLite library never reaches the WASM build.
- The wasm-bindgen API surface is **coarse-grained**: each export does substantial work and returns a complete result. No chatty per-field calls across the JS↔WASM boundary.

**Build commands**

```
just wasm-build         # cargo build for wasm32-unknown-unknown
just wasm-deps-check    # CI gate — fails if rusqlite leaks into wasm32 deps
just wasm-pack-build    # produces .wasm + JS glue via wasm-pack (target=bundler by default)
```

The wasm32 builds use the rustup-managed `stable` toolchain explicitly (the project's default Homebrew rustc lacks the wasm32 sysroot). One-time setup per machine:

```
rustup install stable
rustup target add wasm32-unknown-unknown --toolchain stable
cargo install wasm-pack    # only needed for wasm-pack-build
```

**Current scope (forge-f0gc → forge-nst6)**

- Crate scaffold + buildable WASM artifact (forge-f0gc).
- wa-sqlite binding + `Database` round-trip API + browser-persistent storage (forge-nst6).

`ForgeRuntime` exposes `openDatabase(path)` returning a `Database` handle with `exec` / `query` / `close` methods. The headless-Chrome wasm-pack test harness is deferred to forge-901c; the BrowserStore adapter that consumes `Database` is forge-lu5s.

### wa-sqlite Binding (forge-nst6)

**Distribution channel: bundler-resolved npm.** `crates/forge-wasm/package.json` declares `wa-sqlite@1.0.0` as a `peerDependency`; downstream consumers (the forge-5x2h Svelte integration, the manual smoke harness, future Tauri/Dioxus consumers) install it at the same pin and let their bundler resolve the imports emitted by wasm-bindgen. CDN-loaded was rejected (CSP burden, breaks offline-first, cold-start latency); vendored was rejected (manual upgrade burden). Revisit-after-MVP-2.0 tracked in **forge-zf8i**.

**SQLite VFS: `IDBBatchAtomicVFS` (IndexedDB).** wa-sqlite@1.0.0 (the only version on npm) ships `OriginPrivateFileSystemVFS` which uses `createSyncAccessHandle()` — Worker-only on Chromium/WebKit. The truly main-thread async OPFS VFS (`OPFSAnyContextVFS`) only exists on master HEAD (1.1.1), unpublished. To stay within the npm-distribution decision AND defer the Worker scaffold (forge-73hi), forge-nst6 uses IDB-backed wa-sqlite. From the user's perspective IDB persistence is functionally equivalent to OPFS (survives reload, same StorageManager quota, same "clear site data" semantics). The progressive Svelte → Tauri → Dioxus rewrite is unaffected by VFS choice — the wasm-bindgen surface above the VFS is identical. Migration to OPFS tracked in **forge-n89p**.

**Asyncify-required ESM:** the binding pulls `wa-sqlite/dist/wa-sqlite-async.mjs` (the Asyncify build), required because the IDB VFS uses async I/O internally via wa-sqlite's `handleAsync` pattern. The non-async build (`wa-sqlite.mjs`) only works with sync VFSes (Worker-only OPFS variants).

**Error mapping:** `forge-core` exposes `ForgeError::WasmDatabase(String)` as an always-present variant (no feature gate). It shares the `DATABASE_ERROR` wire code with the rusqlite-gated `Database` variant, so API consumers can't distinguish backends from the JSON shape. Always-present rather than feature-gated because cargo workspace resolver v2 unifies forge-core's features per-target — once forge-wasm's rlib is part of a workspace build, the variant must be reachable in native consumers' exhaustive matches anyway. Cheaper to keep it always-on than to coordinate a feature-passthrough chain through every consumer crate.

**Bundle size (wasm-pack `--target bundler`, release profile):**

| Artifact | Size | Notes |
|---|---|---|
| `forge_wasm_bg.wasm` (forge-f0gc baseline) | ~21 KB | Scaffold only, no binding |
| `forge_wasm_bg.wasm` (forge-nst6) | ~71 KB | +50 KB for the binding shim + Closure runtime |
| `forge_wasm_bg.js` (forge-nst6) | ~22 KB | wasm-bindgen JS glue |
| `wa-sqlite-async.wasm` (peer dep) | ~1.14 MB | The actual SQLite + Asyncify runtime — independent of forge-wasm's growth |
| `wa-sqlite-async.mjs` (peer dep) + IDB VFS module | ~80 KB combined | wa-sqlite JS layer |

**Net first-load cost for browser persistence:** forge-wasm (~93 KB) + wa-sqlite-async (~1.22 MB) ≈ **1.3 MB**, dominated by the SQLite WASM payload. Cacheable across sessions.

**Manual proof-of-concept harness:** `crates/forge-wasm/examples/browser-smoke/` runs the open → CREATE TABLE → INSERT → SELECT → assert round-trip end-to-end via Vite. Used for manual verification until **forge-901c** ships an automated headless-Chrome harness over the same code path.

**Architectural rules (added in nst6):**

- `ForgeError::WasmDatabase(String)` is always-present on forge-core (no feature gate). All consumers that exhaustively match on `ForgeError` must handle this arm. Native consumers route it to the same status code as the rusqlite-gated `Database` variant.
- The wa-sqlite peer-dep version pin in `crates/forge-wasm/package.json` MUST track exactly what the bindings target. Bumping wa-sqlite without updating the Rust binding will silently break.
- Vite (or any consumer bundler) needs `resolve.alias` entries for `wa-sqlite/dist/...` and `wa-sqlite/src/...` because wa-sqlite@1.0.0's package.json has no `exports` field — Rollup's strict subpath resolution refuses these otherwise. The browser-smoke `vite.config.js` documents the canonical alias set; forge-5x2h should reuse it.

## Server (Build-Time)

- Full graph in SQLite/HelixDB
- LLM-assisted curation (batch, not realtime) — propose edges, validate skills
- Embedding generation (can use larger models than browser)
- HNSW index building (one-time, included in snapshot)
- Snapshot serialization and upload to CDN (Cloudflare R2)
- Curation review queue processing

## Browser (Runtime)

### Stack

| Component | Library | Size | Role |
|-----------|---------|------|------|
| Embedding model | transformers.js (all-MiniLM-L6-v2) | ~80MB WASM/WebGPU | Embed query text |
| HNSW index | usearch-js or hnswlib.js | Scales with corpus | Nearest-neighbor lookup |
| Graph structure | Plain JS Map/Set | ~50KB at 10k nodes | Relationship traversal |
| Fuzzy string match | fuse.js or custom trigram | Tiny | Alias/abbreviation matching |
| Storage | IndexedDB | N/A | Cache snapshots between sessions |

### Snapshot Lifecycle

1. Server builds snapshot via `forge_sdk::db::stores::skill_graph::build_structural_snapshot(conn, snapshot_id)` (forge-ubxb).
2. Future: forge-afyg packs the embedding payload and HNSW blob into the binary sections of the same `SkillGraphSnapshot` container.
3. Encode to bytes via `SkillGraphSnapshot::encode()` (custom container — see `graphs/skills.md` for byte layout).
4. Upload to Cloudflare R2 (or equivalent CDN). The CDN's ETag header drives client cache validation.
5. Browser checks ETag on load. If new: download full snapshot → store in IndexedDB keyed by `metadata.snapshot_id` → decode via `SkillGraphSnapshot::decode()` → rebuild in-memory structures.
6. If cached: load from IndexedDB directly.
7. Future: delta/diff-based sync for incremental updates (snapshot version bumps required).

### Snapshot Budget (at 10k skills)

| Component | Size | Status |
|-----------|------|--------|
| Embeddings (10k × 384 × 4 bytes, raw bytes) | ~15MB | Format slot ready (forge-ubxb), payload deferred to forge-afyg |
| Graph structure (JSON header: nodes + typed edges + market stats) | ~3MB | Implemented (forge-ubxb) |
| HNSW index (pre-built, opaque blob) | 0 bytes (MVP) | Format slot stays empty — forge-afyg builds the index at load time, keeping the snapshot library-agnostic during the HNSW lib evaluation |
| **Total budget** | **<25MB** | Verified at 10k synthetic nodes with simulated embeddings |

Acceptable for initial download, cached in IndexedDB thereafter. Compression (gzip/brotli) at the CDN layer would further reduce wire size; not part of the format itself.

### Snapshot Composition

The skill snapshot may include non-skill data needed for runtime operations:
- Org/Industry context (for extraction disambiguation)
- Cert → skill validation mappings (for alignment scoring)
- Archetype → skill expectations (for Career Target, future)

Decision: include in one snapshot (larger, self-contained, full offline) vs separate snapshots (smaller, more requests, partial offline). Recommend: single snapshot for v1, separate per graph for v2.

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Snapshot load from IndexedDB | < 500ms | Deserialization + in-memory structure build. forge-afyg measured ~28ms at 10k×384 (native release); WASM-release expected within 2-3×. |
| Snapshot download from CDN | network-dependent | ~20MB, cached after first download |
| Embedding model init (first load) | 2-5s | WASM compilation, one-time per session |
| Single text embedding | ~50ms | After model init |
| HNSW search (top-k at 10k) | < 50ms | forge-afyg measured ~1ms native release with linear-scan prototype; will only get faster when a real HNSW lib lands. |
| Full JD extraction | < 500ms | All 4 extractors + RRF fusion |
| Alignment scoring (resume vs JD) | < 100ms | Graph traversal + scoring |
| Skill autocomplete (per keystroke) | < 50ms | forge-afyg measured <1ms native release for substring autocomplete (string-only — embedding step adds the ~50ms model latency above). |
| Memory footprint (skill graph runtime, 10k×384) | ~17MB | Embeddings 15MB + petgraph + lookup maps. Independent of the embedding model's ~80MB. |

## Performance Optimizations

- **WebGPU:** If available, embedding is ~10x faster. Detect and prefer.
- **Web Worker:** Extraction pipeline MUST run in a Web Worker to avoid blocking UI thread.
- **Service Worker:** Could pre-cache snapshot for true offline support (future).
- **Lazy model loading:** Don't load embedding model until first extraction is requested.
- **HNSW warm-up:** Pre-load HNSW index on snapshot load, before first query.

## Offline Capability

After initial snapshot download + embedding model cache:
- Extraction: fully offline
- Alignment scoring: fully offline
- Skill search/autocomplete: fully offline
- New skill submission: queued, synced when online
- Snapshot updates: checked when online, stale snapshot still functional
