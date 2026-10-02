# Forge Resume Builder

set dotenv-load := true

# Resolve DB path relative to workspace root (bun --filter changes CWD to package dir)
export FORGE_DB_PATH := absolute_path(env("FORGE_DB_PATH", "./data/forge.db"))

# Cargo via rustup so the stable toolchain is used even when another Rust
# (e.g. Homebrew) comes first on PATH. RUSTC is set explicitly because that
# other rustc can still win the PATH lookup inside `rustup run`.
cargo := "RUSTC=$(rustup which rustc --toolchain stable) rustup run stable cargo"

# ─── Modules ──────────────────────────────────────────────

mod docker ".docker/justfile"
mod test "packages/justfile"
mod data "data/justfile"

# ─── Local development ───────────────────────────────────

# Start core API + MCP server + WebUI locally (SQLite)
dev:
    @echo "Starting Forge API (:3000) + MCP (:5174) + WebUI (:5173)..."
    @echo "Database: {{FORGE_DB_PATH}}"
    bun run --filter '@forge/core' dev &
    sleep 1
    bun run --filter '@forge/mcp' dev &
    sleep 1
    bun run --filter '@forge/webui' dev

# Start dev + MCP Inspector for debugging
debug:
    @echo "Starting Forge + MCP Inspector..."
    @echo "Database: {{FORGE_DB_PATH}}"
    bun run --filter '@forge/core' dev &
    sleep 1
    bun run --filter '@forge/mcp' dev &
    sleep 1
    bun run --filter '@forge/webui' dev &
    sleep 1
    bun run --filter '@forge/mcp' inspect:http

# Build distributable artifacts
release *package="":
    @if [ -z "{{package}}" ]; then \
        bun run --filter '*' build; \
    else \
        bun run --filter "@forge/{{package}}" build; \
    fi

# ─── Individual services (local) ─────────────────────────

# Start only the core API server
api:
    bun run --filter '@forge/core' dev

# Start the Rust API server (forge-server) on :3000 against FORGE_DB_PATH
server:
    {{cargo}} run -p forge-server

# Start only the WebUI dev server (needs 'just api' in another tab)
webui:
    @echo "Note: API server must be running on :3000 (run 'just api' in another tab)"
    bun run --filter '@forge/webui' dev

# Start the MCP server on STDIO
mcp:
    bun run packages/mcp/src/index.ts

# ─── Setup & utilities ───────────────────────────────────

# Install dependencies and set up env
setup:
    bun install
    @test -f .env || cp .env.example .env && echo "Created .env from .env.example"
    @mkdir -p data
    @echo "Done. Run 'just dev' to start."

# Run only the core API tests
test-core:
    @just test core

# Run DB migrations (TypeScript runner)
migrate:
    bun run --filter '@forge/core' migrate

# Check the Rust workspace compiles
check-rust:
    {{cargo}} check --workspace

# Rust formatting + lint, as CI runs them
lint-rust:
    {{cargo}} fmt --all --check
    {{cargo}} clippy --workspace --all-targets -- -D warnings

# ─── Telemetry (mitmproxy → MLFlow) ─────────────────────

# Start mitmproxy telemetry capture for Claude Code
telemetry:
    @echo "Starting mitmproxy telemetry proxy on :8888..."
    @echo "Use with: HTTPS_PROXY=http://localhost:8888 NODE_EXTRA_CA_CERTS=~/.mitmproxy/mitmproxy-ca-cert.pem claude"
    mitmdump --listen-port 8888 -s scripts/claude-telemetry-addon.py

# Ingest captured telemetry into MLFlow
telemetry-ingest:
    MLFLOW_TRACKING_URI=http://127.0.0.1:5000 python3 scripts/ingest-telemetry.py

# Stop the telemetry proxy
telemetry-stop:
    pkill -f mitmdump || true
    @echo "Telemetry proxy stopped."

# ─── forge-wasm (Rust → WebAssembly) ────────────────────
#
# These recipes build the forge-wasm crate for the wasm32-unknown-unknown
# target. They run cargo through `rustup run stable` because a non-rustup
# Rust on PATH (e.g. Homebrew) lacks the wasm32 sysroot. rust-toolchain.toml
# pins the channel and components; `wasm-target` installs the wasm32 target
# (`rustup run` does not read the `targets` list in that file).
# Requirements:
#
#   rustup (https://rustup.rs)
#   cargo install wasm-pack         # only needed for `wasm-pack-build`
#
# Install the wasm32 target for the stable toolchain (idempotent).
wasm-target:
    rustup target add wasm32-unknown-unknown --toolchain stable

# Run any cargo subcommand against wasm32, e.g. `just wasm-cargo check -p forge-sdk`.
wasm-cargo *args: wasm-target
    {{cargo}} {{args}} --target wasm32-unknown-unknown

# Build forge-wasm to wasm32. Use this in CI to catch WASM-only breaks at
# build time rather than at deploy time. forge-server MUST NOT depend on
# forge-wasm (passive guard: forge-wasm is omitted from workspace.dependencies).
wasm-build: wasm-target
    @echo "Building forge-wasm for wasm32-unknown-unknown..."
    {{cargo}} build -p forge-wasm --target wasm32-unknown-unknown

# Verify forge-wasm's wasm32 dep tree stays free of native-only crates
# (rusqlite, libsqlite3-sys, openssl-sys, etc.). Use this as a CI gate to
# catch accidental imports of native deps from the WASM crate.
wasm-deps-check: wasm-target
    @echo "Checking forge-wasm wasm32 deps for native-only crates..."
    @# `cargo tree -i` exits 0 with empty stdout when the crate is absent, so test the output.
    @for krate in rusqlite libsqlite3-sys openssl-sys; do \
        if [ -n "$({{cargo}} tree -p forge-wasm --target wasm32-unknown-unknown -i $krate 2>/dev/null)" ]; then \
          echo "FAIL: $krate found in forge-wasm wasm32 deps"; exit 1; \
        fi; \
      done; \
      echo "OK: no native-only crates in forge-wasm wasm32 deps"

# Negative control for wasm-deps-check: the same probe MUST find rusqlite in forge-sdk.
wasm-deps-check-control:
    @test -n "$({{cargo}} tree -p forge-sdk -i rusqlite 2>/dev/null)" \
      && echo "OK: probe detects rusqlite in forge-sdk (check can fail)" \
      || { echo "FAIL: probe found nothing in forge-sdk; wasm-deps-check is vacuous"; exit 1; }

# Produce a loadable .wasm + JS glue via wasm-pack. Output goes to
# crates/forge-wasm/pkg/. Requires `cargo install wasm-pack`.
wasm-pack-build target="bundler":
    @echo "Running wasm-pack build (target: {{target}})..."
    cd crates/forge-wasm && wasm-pack build --target {{target}}

# ─── Extension packaging ────────────────────────────────

# Package extension for store submission (both browsers)
pack-extension:
    cd packages/extension && bun run build
    @mkdir -p dist
    cd packages/extension/dist/chrome && zip -r ../../../../dist/forge-job-tools-chrome-v$(cd ../.. && jq -r .version manifest.json).zip .
    cd packages/extension/dist/firefox && zip -r ../../../../dist/forge-job-tools-firefox-v$(cd ../.. && jq -r .version manifest.firefox.json).zip .
    @echo "Packaged:"
    @ls -la dist/forge-job-tools-*.zip

# Package source for Firefox AMO review (required for bundled code)
pack-extension-source:
    git archive HEAD --prefix=forge-source/ -o dist/forge-job-tools-source.zip
    @echo "Source archive: dist/forge-job-tools-source.zip"
