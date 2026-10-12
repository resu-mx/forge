# Resume tagline: when it regenerates, and what counts as blank

> Status: Decided 2026-10-12. Closes the questions in resu-mx/forge#72 (part of #66, the tagline
> Rust port). Behavior changes below landed in TS first, with route tests, then in Rust.

A resume has two stored tagline columns:

- `generated_tagline`: built from the JDs linked to the resume (a TF-IDF pass over their text,
  boosted by the skills in the library), prefixed with the resume's `target_role`. It is derived
  data, and regeneration is the only writer. `NULL` when nothing is linked.
- `tagline_override`: written by a person (`PATCH /resumes/:id/tagline-override`). Regeneration
  **never** writes it.

Code: TS `packages/core/src/services/tagline-service.ts`; Rust `crates/forge-sdk/src/services/tagline/`.
Both runtimes serve one contract (`just parity-tier0`; the route tests in
`packages/core/src/routes/__tests__/resume-tagline.test.ts` and `jd-resume-linkage.test.ts` are
the reference).

## What regenerates the generated tagline

| Event | Regenerates? | Since |
|---|---|---|
| A JD is linked to the resume (new link or a repeat) | yes | Phase 92; Rust in #71 |
| A JD is unlinked from the resume | yes | Phase 92; Rust in #71 |
| `POST /resumes/:id/tagline/regenerate` (the Regenerate button) | yes | Phase 92 |
| A linked JD is **deleted** | **yes**, for each resume that was linked | #72 |
| The resume's `target_role` **changes** | **yes** | #72 |
| A JD's `raw_text` is edited | no (manual) | decided, unchanged |
| A skill is added, renamed or merged | no (manual) | decided, unchanged |

## The decisions

### 1. A JD's `raw_text` changes: keep TS (manual Regenerate)

Options: keep TS; regenerate every linked resume; flag the tagline as stale in the UI.

Chosen: **keep TS**. A JD can be linked to many resumes, an edit is often a typo fix, and the
Regenerate button is one click. A stale flag would need a migration both runners apply, for a
case a person can already fix. The cost is that the AUTO tagline can lag a JD edit until someone
regenerates; that is accepted.

### 2. A JD is deleted: regenerate the affected resumes, after the delete

Options: keep TS; regenerate before the delete; regenerate after it.

Chosen: **regenerate after the delete, for each resume that was linked.** The links cascade away
with the JD, so a tagline built before the delete would still quote it, and a tagline left alone
quotes a JD that no longer exists, with nothing on screen to explain why. The handler reads the
linked resume ids first (the cascade removes them), deletes the JD, then regenerates each. In
Rust the delete and the regenerations are one transaction. An unknown JD is a 404 and
regenerates nothing. A resume's override is never touched.

### 3. `target_role` changes: regenerate when it actually changes

Options: keep TS; regenerate on update; build the prefix at render time.

Chosen: **regenerate on update, only when the role differs from the stored one.** The prefix is
the most visible part of the tagline, and a stale one ("Old Role -- kafka + ...") looks wrong in
the header and the export. Building the prefix at render time would make the stored value stop
meaning what it means everywhere else, so it was not chosen. A patch that leaves the role out,
or sets it to the same value, does not regenerate. A resume with no linked JD keeps
`generated_tagline` as `NULL`; there is nothing to prefix. In Rust the update and the
regeneration are one transaction.

### 4. Skills change: keep TS (manual Regenerate)

Options: keep TS; regenerate every resume; leave it manual.

Chosen: **keep TS.** Regenerating on a skill change costs resumes x JDs per skill edit, and it
would quietly change rankings on resumes nobody opened. The skill boost is a refinement, and
Regenerate applies the current library when someone wants it.

### 5. `resolved` and blank overrides: a blank override is no override

Question: a whitespace-only `tagline_override` was returned as `resolved` while `has_override`
was `false`, so the two disagreed.

Options: keep `??`; treat a blank override as absent in `resolved` too.

Chosen: **a blank override is absent.** `resolved` is a non-blank override, else a non-blank
generated tagline, else `""`; `has_override` is true exactly when the override is non-blank. The
stored text is not changed or trimmed (`tagline_override` is still reported as stored). A blank
override can only come from a row written directly: the override writer already stores blank as
`NULL`.

### 6. Empty-string columns in the compilers: the same rule

Question: TS skipped `""` by truthiness (`resume-compiler.ts`); Rust rendered `Some("")`
(`compiler_service.rs`); and both used a whitespace-only override as the header tagline.

Chosen: **one rule in both compilers: an empty or whitespace-only value is absent**, at every
step of the header's fallback chain: `tagline_override`, then `generated_tagline`, then the legacy
`header.tagline`, then `target_role`. A real value is used as stored, untrimmed. The rule is
`nonBlank` in TS and `non_blank` in Rust (JS whitespace semantics, so U+FEFF is blank and
U+0085 is not). The web UI's edit box follows it too: it never opens on a blank override.

## Out of scope

The Phase 92 plan's "prompt the user to accept or keep their override" after a regeneration was
never built (`@forge/sdk` drops the link response's `tagline`). That is a UI feature, not a parity
question.

## Where this is tested

- `resume-tagline.test.ts` (both runtimes): blank overrides in `GET /resumes/:id/tagline` and in
  the rendered header and export; `PATCH /resumes/:id` with a changed, unchanged and absent
  role; `DELETE /job-descriptions/:id` regenerating linked resumes only.
- `jd-resume-linkage.test.ts` (both runtimes): the Phase 92 link and unlink regeneration.
- Rust: `crates/forge-api/tests/tagline.rs`, `jd_resume_links.rs`, and the unit tests in
  `crates/forge-sdk/src/services/tagline/mod.rs` and `compiler_service.rs`.
