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
  in `PAGES_PROJECT` (`resumx-webapp`, which serves app.resu.mx) with `wrangler pages deploy`:
  production on `main`, a `pr-<N>` preview for same-repo PRs. It runs only while the repository
  variable `PAGES_DEPLOY_ENABLED` is `true`; set it to `false` to stop deploying.
  It reads the Cloudflare token and account id from 1Password through the `cf-pages-app`
  environment, which needs the secrets `OP_SERVICE_ACCOUNT_TOKEN` and `OP_ENVIRONMENT_ID_CF_PAGES`.
- The Pages project, hostname and DNS are owned by Terraform in `resu-mx/infra`. Cloudflare no
  longer builds the app: the Git-integrated `resumx-app` project and its `scripts/pages-build.sh`
  build command are gone.

### `demo-data.yml`

**Opt-in, never a required check:** it runs only when someone asks. It generates the demo
datasets (`packages/demo-data`, see `docs/src/dev/demo-datasets.md`), and can test them against
R2 and the browser app and publish them.

- **To run it:** `gh workflow run demo-data.yml -f publish=false` (add `-f e2e=true` for the
  browser spec, `--ref <branch>` for a branch other than `main`), or add the `demo-data` label to
  a PR. A label run uses the PR as it was when labeled; remove and re-add the label to test newer
  commits. Any other label starts a run whose jobs all skip.
- **`generate + verify`** needs no secrets. It runs `just demo-data generate` and
  `just demo-data verify` (including the ignored Rust SDK test), then uploads `data/demo` as the
  `demo-datasets` artifact (kept 3 days).
- **`R2 round trip + preview push`** runs in the `r2-preview` environment, for dispatches and
  same-repo PRs only (fork PRs get no secrets). It runs the live round-trip test, which refuses
  any bucket not ending in `-preview`, then pushes the artifact to the preview bucket with every
  push guard.
- **`demo datasets in the browser (Playwright)`** runs for `e2e=true` or a label run. It builds
  the wasm bundles like `rust.yml`'s browser job and runs `e2e/wasm/demo-datasets.spec.ts` against
  the artifact. The Playwright report is uploaded on failure.
- **`publish to the production bucket`** runs only for a dispatch from `main` with
  `publish=true`, never for a PR. It waits for the `r2-prod` reviewers (that environment allows
  only `main`), runs after the round trip passes and after e2e passes when it was requested, then
  runs `publish --yes`.
- Both environments hold the secret `OP_SERVICE_ACCOUNT_TOKEN` and the variable
  `OP_ENVIRONMENT_ID`. `aRustyDev/load-secrets-action` (the pin from `app-build.yml`) exports the
  `R2_*` credentials from 1Password, masked. The workflow checks only that they are set and
  prints their lengths.

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
- `demo-data.yml` is opt-in on purpose. Never add it to the required checks, and never let its
  `publish` job run for a `pull_request` event.
