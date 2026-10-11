# `packages/demo-data`: generated demo datasets

Builds one SQLite file per persona through the real Rust API, marks it as generated, and
verifies it. Output: `data/demo/user/<uuid>/{data.sqlite,manifest.json}` plus
`data/demo/index.json` (gitignored). `src/r2/` pushes and pulls them to and from R2; the
marker contract, key layout and push guards are in `docs/src/dev/demo-datasets.md`.

## Run

| Command (repo root) | Does |
|---|---|
| `just demo-data generate [persona\|all]` | `cargo build -p forge-server`, then generate. Flags: `--seed`, `--as-of`, `--out`, `--server-bin`, `--keep-temp` |
| `just demo-data verify [persona\|all]` | re-check written files: sha256, compaction, `dataset_meta`, invariants, counts, fingerprint; then `crates/forge-sdk/tests/demo_datasets.rs` (ignored by default) opens every dataset with the Rust SDK: no pending migration, no foreign-key violation, `kind=generated` |
| `just demo-data e2e [playwright flags]` | build the wasm bundles, then `packages/webui/e2e/wasm/demo-datasets.spec.ts`: import each dataset into the browser app (the first through Settings → Storage, the rest through `forgeRuntime.importDatabase`) and check the dashboard, every board column, the PDF preview and the other pages. Fixed port 5198: one run at a time |
| `just demo-data test` | `bun test`; the integration test skips unless `FORGE_SERVER_BIN` or `target/debug/forge-server` exists |
| `just demo-data push-preview [persona\|all]` | guarded upload to the preview bucket (`--force`, `--allow-stale`) |
| `just demo-data publish [persona\|all]` | the same to the **production** bucket, after a confirmation prompt |
| `just demo-data pull\|head <uuid\|slug> <preview\|prod>`, `ls <preview\|prod>` | download and verify, show metadata, list |
| `just demo-data roundtrip` | live put/head/list/get/delete test against the preview bucket (`FORGE_R2_ROUNDTRIP=1`) |

The R2 recipes load `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY` with `op run`
(private `_r2` recipe). Never print their values, and never point the round trip or a test at
the production bucket. `src/r2/__tests__/sign.test.ts` covers signing and every push guard offline.

CI: `.github/workflows/demo-data.yml` is opt-in (dispatch, or the `demo-data` PR label). It runs
generate and verify, the round trip and a preview push, the e2e spec, and, on request from
`main`, `publish`. It calls `bun test` and `bun run src/cli.ts` directly with the `R2_*`
variables already exported, so `op` is never involved. See `.github/AGENTS.md`.

Defaults are fixed (`--as-of 2026-09-30T17:00:00Z`, `--seed forge-demo-v1`), never the wall clock.
`verify`'s Rust check and `e2e` read `FORGE_DEMO_DATA_DIR` (default `data/demo`): set it when you
pass `--out`. Both test every `user/*/data.sqlite` there, whatever the persona argument.

## How a dataset is made (`src/generate/index.ts`)

1. Spawn `forge-server` on a temp file (`src/server.ts`, copied from core's route-test helper).
2. Drive phases through `ForgeClient`, plus raw `fetch` (`src/api.ts`) where the SDK lacks a
   route or field: reference data and the **whole skill catalog first** (a derivation invents a
   lower-case `tool` skill for any unknown technology), profile and answer bank, orgs,
   qualifications, sources, bullets and perspectives via `derivations.prepare`/`commit`,
   summaries, JDs, contacts, resumes (tagline regenerated after all JD links), notes, API checks.
3. Stop the server (SIGINT), then the post-pass (`src/postpass/`): status overlay, timestamp
   backdating, `dataset_meta`, invariants, compaction, manifest.

## Rules

- Packages may not import `@forge/core` code. Depend on `@forge/sdk` only; read migrations from
  disk when a test needs a schema (`src/__tests__/helpers.ts`).
- The repo and the datasets are public. Everything is fictional: `@example.com` emails,
  `example.*`/`*.test` hosts, `(NNN) 555-01NN` phones, invented organizations (a denylist test
  catches real ones). Real cities and states are fine. `src/conventions.ts` is the check; it
  runs over the corpus and every TEXT column of the output.
- Corpus times are `daysAgo` offsets from `--as-of`; calendar dates are `monthsAgo`. The PRNG
  (`src/prng.ts`) only jitters timestamps; no faker.
- Determinism is measured by `content_fingerprint` (ids are random, so sha256 differs per run).
  Its natural keys come from the data, so titles, bullet and perspective contents, names, etc.
  must be unique within a persona (`corpus.test.ts` enforces it).
- Keep each persona under ~150 bullets and perspectives (Rust list endpoints clamp at 200).
- Persona uuids (`src/ids.ts`) are R2 keys: never change the namespace or a slug.

## The post-pass sets what no API can

Each is transition-checked and logged in the manifest's `overlay`:

- source statuses other than `draft` (no route updates a source's status);
- `approved → archived` for bullets and perspectives (no archive route);
- note references to credentials and certifications (the Rust API refuses those entity types).

Backdating covers every `*_at` column from the ledger or a rule (`TIMESTAMP_RULES`); a column
with neither fails the run, as does a table with ids but no natural-key rule in
`src/postpass/fingerprint.ts`. A migration that adds either must update those lists.

## Adding a persona

Add `src/personas/<slug>/` (typed corpus + `jds/*.md`), register it in `src/personas/index.ts`,
add its size bands to `corpus.test.ts`, and run `just demo-data test`.
