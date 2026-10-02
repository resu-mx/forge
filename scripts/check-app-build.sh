#!/usr/bin/env bash
# Checks a built app (default packages/webui/build) before it is published to Cloudflare Pages.
# Used by scripts/pages-build.sh (the Pages build) and by .github/workflows/app-build.yml.
set -euo pipefail

dir="${1:-packages/webui/build}"
# Cloudflare Pages rejects any single asset over 25 MiB. The Typst module is the large one.
max_bytes="${MAX_FILE_BYTES:-26214400}"

echo "wasm assets:"
find "$dir" -type f -name '*.wasm' -exec ls -l {} \; | awk '{print "  " $5, $9}'

over=$(find "$dir" -type f -size +"${max_bytes}c")
if [ -n "$over" ]; then
  echo "FAIL: files over ${max_bytes} bytes (Pages per-asset limit):"
  echo "$over"
  exit 1
fi

for f in index.html _headers _app/version.json; do
  test -f "$dir/$f" || { echo "FAIL: $dir/$f is missing"; exit 1; }
done

# Both modules must be present, or the app would build "successfully" without its runtime.
n=$(find "$dir" -name '*.wasm' | wc -l | tr -d ' ')
[ "$n" -ge 2 ] || { echo "FAIL: expected the data-layer and Typst wasm, found $n"; exit 1; }

echo "OK: $dir is within limits and has the expected shape"
