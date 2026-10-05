# `.github/`: CI and release workflows

## Workflows

### `rust.yml`

Runs on every PR and on pushes to `main`. Jobs:

- **check + test (native):** `cargo check` and `cargo test --workspace --locked`.
- **forge-wasm (wasm32):** `just wasm-build`, a forge-typst wasm build, `just wasm-deps-check`,
  and the check's must-fail control.
- **TS route tests vs Rust server:** `just parity-tier0`.
- **runtime + SDK tests (bun):** `packages/runtime` typecheck and tests, plus `packages/sdk`
  tests.
- **core loop in the browser (Playwright):** builds the wasm bundles, installs Chromium and runs
  `packages/webui/e2e/wasm` against the production build.
- **fmt + clippy (advisory):** reports problems but does not block.

### `app-build.yml`

Runs on every PR and on pushes to `main`. Builds the browser-first app (wasm bundles plus the
static web UI) with cached Rust, wasm-bindgen and bun, runs `scripts/check-app-build.sh`, and
uploads `packages/webui/build` as the `app-static` artifact.

- **`build static app`** always runs.
- **`deploy to Cloudflare Pages`** uploads the artifact to the direct-upload Pages project named
  in `PAGES_PROJECT` with `wrangler pages deploy`: production on `main`, a `pr-<N>` preview for
  same-repo PRs. It runs only when the repository variable `PAGES_DEPLOY_ENABLED` is `true`.
  It reads the Cloudflare token and account id from 1Password through the `cf-pages-app`
  environment, which needs the secrets `OP_SVC_ACCT_TOKEN` and `OP_ENVIRONMENT_ID_CF_PAGES`.
- The Pages project, hostname and DNS are owned by Terraform in `resu-mx/infra`. Until the
  domain moves to the direct-upload project, the Git-integrated `resumx-app` project still builds
  itself with `scripts/pages-build.sh`.

### `extension-publish.yml`

Runs on a `v*` tag push, or a manual dispatch with `tag`, `skip_chrome` and `skip_firefox`
inputs. It runs in this order:

1. Check the tag against the manifest version.
2. Test, build and zip the extension.
3. Create a GitHub release (tag pushes only).
4. Publish to the Chrome Web Store and Firefox AMO.

## Conventions

- `rust.yml` has **no `paths:` filter, on purpose**. A required check that a path filter skips
  never reports, and that blocks the PR. Do not add one.
- Lint stays advisory (`continue-on-error`) until the workspace is rustfmt-clean and the
  `todo!()` stubs are gone. Then drop `continue-on-error` to make it blocking.
- CI pins Bun `1.4.2` and installs with `bun install --frozen-lockfile`. Commit `bun.lock`
  together with any `package.json` change. Cargo runs with `--locked`, so commit `Cargo.lock`
  changes too.
- Only `runtime` and `sdk` among the TS packages run in CI (the parity job also exercises
  core's route tests). Run the rest locally with `just test`.
- An extension release tag `v<version>` must equal `version` in
  `packages/extension/manifest.json`, so bump both manifests before tagging (see
  `packages/extension/AGENTS.md`).
- Store credentials come from 1Password environments through `aRustyDev/load-secrets-action`
  (`OP_SVC_ACCT_TOKEN`, `OP_ENVIRONMENT_ID_*` secrets). Never put store credentials in the
  repo or in workflow `env:` literals.
