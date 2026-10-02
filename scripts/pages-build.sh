#!/usr/bin/env bash
# Build command for the Cloudflare Pages project `resumx-app` (app.resu.mx), configured in
# resu-mx/infra (terraform/cf/pages.tf). Pages has no Rust, so this installs the toolchain,
# compiles the WASM bundles, builds the static UI and checks the result.
#
# Pages runs it after `bun install`, with a 20-minute limit. It is also runnable by hand on a
# Linux x86_64 machine. Every step prints what it found, so a failed build names its cause.
set -euo pipefail
cd "$(dirname "$0")/.."

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
step() { printf '\n==> %s\n' "$*"; }

step "environment"
uname -srm
echo "bun: $(bun --version 2>&1 || echo missing)"
echo "node: $(node --version 2>&1 || echo missing)"
echo "whoami: $(id -un) (uid $(id -u))"

step "rust toolchain"
if ! command -v rustup >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
# Explicit, so rust-toolchain.toml's rustfmt and clippy components are not downloaded for a build.
rustup toolchain install stable --profile minimal --target wasm32-unknown-unknown
export RUSTUP_TOOLCHAIN=stable
rustc --version

step "just"
if ! command -v just >/dev/null; then
  mkdir -p "$HOME/.local/bin"
  curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to "$HOME/.local/bin"
fi
just --version

# wasm-bindgen-cli must match the wasm-bindgen crate in Cargo.lock exactly.
step "wasm-bindgen-cli"
wbg=$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[^0-9.]/, "", $0); print; exit}' Cargo.lock)
[ -n "$wbg" ] || { echo "could not read the wasm-bindgen version from Cargo.lock"; exit 1; }
echo "need wasm-bindgen $wbg"
if ! { command -v wasm-bindgen >/dev/null && [ "$(wasm-bindgen --version | awk '{print $2}')" = "$wbg" ]; }; then
  mkdir -p "$HOME/.local/bin"
  base="https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$wbg"
  tarball="wasm-bindgen-$wbg-x86_64-unknown-linux-musl"
  if curl -fsSL "$base/$tarball.tar.gz" | tar -xz -C /tmp; then
    install -m 0755 "/tmp/$tarball/wasm-bindgen" "$HOME/.local/bin/wasm-bindgen"
  else
    echo "prebuilt binary unavailable; building from source (slow)"
    cargo install wasm-bindgen-cli --version "$wbg" --locked
  fi
fi
wasm-bindgen --version

# SQLite is compiled from C for wasm32, so the C compiler needs the WebAssembly backend.
step "clang with the wasm32 backend"
has_wasm_clang() { command -v clang >/dev/null && clang -print-targets 2>/dev/null | grep -q wasm32; }
if ! has_wasm_clang; then
  echo "clang: $(command -v clang || echo none); trying apt"
  if [ "$(id -u)" = 0 ]; then
    apt-get update -qq && apt-get install -y -qq clang llvm
  elif command -v sudo >/dev/null; then
    sudo apt-get update -qq && sudo apt-get install -y -qq clang llvm
  fi
fi
if ! has_wasm_clang; then
  echo "FAIL: no clang with a wasm32 target. Found: $(command -v clang || echo none)"
  clang -print-targets 2>&1 | head -20 || true
  exit 1
fi
clang --version | head -1

step "wasm bundles"
just wasm-bundle typst-bundle

step "web UI"
VITE_FORGE_MODE=wasm bun run --filter '@forge/webui' build

step "checks"
bash scripts/check-app-build.sh packages/webui/build
