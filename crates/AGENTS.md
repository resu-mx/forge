# `crates/`: Rust workspace

The TypeScript → Rust port, browser-first. One API runs natively (`forge-server`) and inside a
browser Worker over SQLite on OPFS (`forge-wasm`). The TypeScript stack in `packages/` is
still the primary runtime.

- Roadmap and milestone status: `.agents/plans/rust-mvp-roadmap.md`
- Decisions: `docs/src/dev/adrs/rust-wasm/`
- Original phase spec: `.agents/plans/forge-resume-builder/refs/specs/2026-04-23-rust-implementation-phases.md`

## Crates

| Crate | Owns | Forge deps |
|---|---|---|
| `forge-core` | types, enums, shapes. **No logic, no I/O** | none |
| `forge-sdk` | stores (repositories), services, the migration runner, the IR compiler | core |
| `forge-ai` | LLM prompts, response validation, text analysis. **Never calls an LLM API** | none |
| `forge-typst` | Typst → PDF in-process: no subprocess, no system fonts (bundled in `fonts/`) | core, sdk |
| `forge-api` | the axum `Router` that serves the TS JSON contract; transport-agnostic | core, sdk, ai, typst (`pdf`) |
| `forge-server` | native host for `forge-api` (tokio + hyper) | api (`tokio`, `pdf`), sdk, core |
| `forge-wasm` | browser runtime: `forge-api` + `forge-sdk` over SQLite/OPFS, plus the skill graph and alignment engine | core, sdk, api |
| `forge-mcp`, `forge-cli` | placeholders (`todo!()` binaries) | sdk, core |

The `SPEC.md` files in `forge-core` and `forge-sdk` map each module to its TypeScript source.
`forge-sdk` embeds the TS SQL migrations (`packages/core/src/db/migrations/`) with
`include_str!`, so a new migration there reaches both runtimes.

## Invariants

- `forge-core` features are opt-in (`default = []`). The workspace declares it with
  `default-features = false`, and each consumer enables `rusqlite` explicitly.
- `forge-api` must build for wasm32 without tokio. Native-only dependencies go behind a feature
  (`tokio` and `pdf`, which only `forge-server` enables).
- `forge-server` MUST NOT depend on `forge-wasm`. `forge-wasm` is deliberately left out of
  `[workspace.dependencies]`, so depending on it takes an explicit path dependency that stands
  out in review.
- No native-only crates in the wasm32 graph: `mio`, `libsqlite3-sys`, `openssl-sys`. Check with
  `just wasm-deps-check`; `just wasm-deps-check-control` proves the probe can fail. rusqlite
  itself *is* expected there, because it reaches the browser through `sqlite-wasm-rs`.
- The `forge-wasm` wasm-bindgen surface is coarse-grained: each export does substantial work
  and returns a complete result. It runs in a dedicated Worker, with one database owner per
  origin (ADR 0002).
- The API contract is the TypeScript one (`packages/core/src/routes`). `just parity-tier0` is
  the check. Widen its file list as more of the API is ported.

## Build and test

`rust-toolchain.toml` pins stable plus `wasm32-unknown-unknown`, rustfmt and clippy. The
justfile runs cargo through rustup, so a Homebrew Rust earlier on `PATH` (which has no wasm32
sysroot) is never used.

| Command | Does |
|---|---|
| `just check-rust` | `cargo check --workspace` |
| `just test rust` | `cargo test --workspace`: inline `#[cfg(test)]` modules plus `tests/` in api, server and typst |
| `just lint-rust` | `cargo fmt --check` + `clippy -D warnings` (advisory in CI for now) |
| `just server` | run `forge-server` on :3000 against `FORGE_DB_PATH` |
| `just parity [files]` / `just parity-tier0` | the TS route tests, run against the Rust server |
| `just wasm-build` | build `forge-wasm` for wasm32 |
| `just wasm-cargo <args>` | any cargo subcommand for wasm32 |
| `just wasm-bundle`, `just typst-bundle` | release-size wasm + JS glue into `packages/runtime/pkg` and `typst-pkg` |

Gotchas:

- On macOS, compiling SQLite for wasm32 needs a clang with the WebAssembly backend. Run
  `brew install llvm`, then set `LLVM_BIN=/opt/homebrew/opt/llvm/bin`.
- The bundle recipes need `wasm-bindgen-cli` at the exact `wasm-bindgen` version in
  `Cargo.lock`.
- CI is `.github/workflows/rust.yml` (see `.github/AGENTS.md`).
