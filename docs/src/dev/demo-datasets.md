# Demo datasets

> Status: Implemented. The generator and the R2 tooling live in `packages/demo-data`
> (resu-mx/forge#182; R2 push/pull: #189).

A demo dataset is a complete Forge database for one fictional persona, built through the real
Rust API by `packages/demo-data` and marked as generated. Datasets are stored in Cloudflare R2
and reach the browser app through the existing Settings → Storage import. No Worker serves them.
This page covers the marker, the object layout, the operator flow and the push guards. How the
generator works is in `packages/demo-data/AGENTS.md`.

## The marker contract

A dataset carries its marker in two places, and the two must agree.

**In the database:** the `dataset_meta` table (migration `055_dataset_meta`) holds key/value
rows. Real user databases leave it empty. `kind = 'generated'` is the marker itself; the other
rows record where the file came from.

| `dataset_meta` key | Value |
|---|---|
| `kind` | always `generated` |
| `generator`, `generator_version` | `@forge/demo-data` and its package version |
| `seed`, `as_of` | the generator inputs (`as_of` is the fixed "now" of the dataset) |
| `persona`, `dataset_uuid` | the persona slug and its uuid (see the key layout below) |
| `generated_at` | when the file was written |
| `schema_head` | the newest migration applied to the file, e.g. `055_dataset_meta` |

**On the R2 object:** `x-amz-meta-*` metadata repeats the marker and adds the file's hashes, so
`head` can say what an object is without downloading it. Every value is printable ASCII, and
the whole set stays well under R2's 8 KiB metadata limit. The object's `Content-Type` is
`application/vnd.sqlite3`.

| Metadata | Source |
|---|---|
| `forge-kind`, `forge-generator`, `forge-generator-version`, `forge-seed`, `forge-persona`, `forge-dataset-uuid`, `forge-as-of`, `forge-generated-at`, `forge-schema-head` | the matching `dataset_meta` row (the manifest must agree) |
| `forge-sha256` | sha256 of the file's bytes |
| `forge-content-fingerprint` | sha256 of a canonical dump with ids replaced by natural keys. Ids are random, so this is what stays the same across regenerations. |

## Key layout

```
<bucket>/user/<uuid>/data.sqlite
```

- `<uuid>` is `UUIDv5(slug, UUIDv5("demo-data.resu.mx", DNS))` (`src/ids.ts`). It is stable
  across seeds and generator versions, so regenerating a persona targets the same key. Never
  change the namespace or a slug.
- Only a canonical lowercase uuid makes a key: `userKey()` rejects anything else (uppercase,
  braces, extra path segments).
- The generator writes the same layout locally: `data/demo/user/<uuid>/{data.sqlite,manifest.json}`
  plus `data/demo/index.json`. `data/demo/` is gitignored.

| Bucket | Use | Lifecycle |
|---|---|---|
| `resumx-user-data` | production: the datasets people import | none |
| `resumx-user-data-preview` | operator checks and the round-trip test | objects expire after 3 days |

`R2_BUCKET_PROD` and `R2_BUCKET_PREVIEW` override the bucket names. Both buckets are managed in
the infra repo.

## Operator flow

From the repo root:

```bash
just demo-data generate early-career-developer    # build forge-server, generate into data/demo/
just demo-data verify                             # re-check hashes, compaction, marker, invariants
just demo-data push-preview early-career-developer
just demo-data head early-career-developer preview
just demo-data pull early-career-developer preview   # → data/demo/pulled/preview/<slug>.sqlite
just demo-data roundtrip                          # live put/head/list/get/delete test (preview only)
just demo-data publish early-career-developer     # PRODUCTION; asks for confirmation
just demo-data ls preview
```

The persona argument defaults to `all`. `push-preview` and `publish` accept `--force` (upload
even when the remote copy is current) and `--allow-stale` (accept a file whose schema head is
older than the repo's). `pull` accepts `--out FILE` and `--force` (overwrite an existing file).

`pull` prints the import steps: in the app, open Settings → Storage, optionally export a backup,
choose the file under "Restore or move in", and confirm "Replace data". The app migrates an
older file on import. Use a separate browser profile for demos, because the import replaces
everything in that profile.

## Credentials

The R2 recipes run the CLI through a private `_r2` recipe. It loads `R2_ACCOUNT_ID`,
`R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY` from a 1Password Environment with
`op run --environment <id> -- …`. If the installed `op` lacks `--environment`, it falls back to
`op environment read` into a temporary `0700` directory that is removed on exit, then
`op run --env-file`.

- **Environment id:** taken from `FORGE_R2_OP_ENV_ID` (`FORGE_R2_OP_ENV_ID_PROD` for production);
  otherwise read from the `OP_ENVIRONMENT_ID` variable of the GitHub Environment `r2-preview` or
  `r2-prod`. The id is not a secret.
- **CI:** when all three `R2_*` variables are already set (for example by a secrets action), `op`
  is skipped. The opt-in workflow `.github/workflows/demo-data.yml` (a manual dispatch, or the
  `demo-data` label on a PR) loads them with `aRustyDev/load-secrets-action` from the `r2-preview`
  and `r2-prod` GitHub Environments. It runs the round trip and a preview push, and runs
  `publish --yes` only for a dispatch from `main` with `publish=true`, after an `r2-prod`
  reviewer approves. `.github/AGENTS.md` describes its jobs.
- **Never printed:** credential values never reach argv and are never printed. Error messages
  name variables, not values, and the in-process credentials object redacts itself from logs,
  `JSON.stringify` and `Bun.inspect`.
- **One token, both buckets:** a single R2 API token (Object Read & Write) covers both buckets.
  The production GitHub Environment adds reviewer protection.

## Push guards

`push` and `publish` run every check on the exact bytes they would upload, before any network
call. A failed check refuses the persona and uploads nothing.

| Guard | Why |
|---|---|
| The file starts with the `SQLite format 3` header | not some other file |
| Header bytes 18–19 are `1,1` (DELETE journal mode) | the browser import drops pages still in a WAL |
| The file's sha256 and size match `manifest.json` | the file has not changed since generation |
| The key's uuid is a registered persona uuid, and the manifest's `uuid` and `r2_key` match the key | nothing outside the demo namespace can be written |
| A `dataset_meta` table exists with `kind = 'generated'` | **a real user database never gets uploaded** |
| `dataset_meta.dataset_uuid` equals the key's uuid, and `persona` owns that uuid | the marker belongs to this key |
| `dataset_meta.schema_head` equals the file's newest migration, the manifest and the repo's newest migration file | the app can open it. `--allow-stale` relaxes only the repo comparison |
| `PRAGMA quick_check` is `ok` | not corrupt |

After the guards pass, the CLI reads the remote object's metadata:

- **Already current:** if the remote `forge-content-fingerprint` and `forge-schema-head` already
  match, the upload is skipped (`--force` uploads anyway).
- **Not ours:** if an object exists at the key without `forge-kind=generated`, the push is
  refused (`--force` overrides).
- **New key:** the put sends `If-None-Match: *`, so two concurrent pushes cannot silently
  overwrite each other.
- **Every put** sends the body's real `X-Amz-Content-Sha256`, so R2 rejects a corrupted body.
  Afterwards a HEAD confirms the size and every metadata value.
- **Production:** `push` targets the preview bucket only. `publish` is the only way to write to
  production; it requires `--yes`, which the `just demo-data publish` confirmation prompt
  supplies.

## Pull verification

`pull <uuid|slug>` downloads the object and writes nothing unless all of these hold:

- the object's `forge-kind` is `generated`;
- its bytes hash to `forge-sha256`;
- `forge-dataset-uuid` matches the key;
- the file is in DELETE journal mode, its `dataset_meta` says `generated` for that uuid, and
  `integrity_check` passes.

The file is written to a temporary name and renamed into place. An existing destination is
kept unless you pass `--force`.

## Round-trip test

`src/r2/__tests__/roundtrip.test.ts` runs only with `FORGE_R2_ROUNDTRIP=1` and credentials, which
`just demo-data roundtrip` provides. If `FORGE_R2_ROUNDTRIP` is set but the credentials are
missing, the test fails instead of skipping.

**Safety:**

- It refuses any bucket whose name does not end in `-preview`, checking before every request.
- It writes only under a fresh random `user/<uuid>/data.sqlite`, tagged with `forge-test-run`.
- It always deletes its keys in `afterAll`; the preview lifecycle rule is the backstop.

**Steps:**

1. Put with `If-None-Match: *`.
2. A second create-only put must fail with `412`.
3. A body whose `X-Amz-Content-Sha256` is wrong must be rejected. This is the control that
   proves the payload hash is checked.
4. HEAD returns the same metadata and size.
5. LIST finds the key.
6. GET returns bytes with the same sha256; opened read-only, the file has the marker and passes
   `integrity_check`.
7. Delete, after which HEAD returns nothing.
8. A wrong secret gives a `403` whose error message contains no credential.

## R2 behaviour this relies on

Verified against the preview bucket on 2026-10-10:

| Behaviour | Observed |
|---|---|
| `PUT` with `If-None-Match: *` on an existing key | `412 PreconditionFailed` |
| `PUT` whose `X-Amz-Content-Sha256` does not match the body | `400 XAmzContentSHA256Mismatch`, nothing stored |
| `PUT` with a wrong `x-amz-checksum-sha256` | `400 BadDigest` (not used; the content hash is enough) |
| `x-amz-meta-*` set on PUT | returned unchanged by HEAD and GET |
| `Content-Type` | returned as stored |

`Bun.S3Client` can neither set nor read `x-amz-meta-*`, so `put`, `head`, `get` and `delete`
are signed with `aws4fetch`. `list` uses `Bun.S3Client`.

## Schema skew

A dataset older than the deployed app is fine, because the import migrates it. A dataset newer
than the app may not open. That is why `forge-schema-head` is recorded and the push guard
compares it to the repo's newest migration. When a migration lands, regenerate and republish.
