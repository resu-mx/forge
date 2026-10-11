# `docs/`: durable reference

Plain Markdown with no build step (no mdBook, no `SUMMARY.md`). `docs/` records conclusions;
work in progress lives in `.agents/plans/`. When a plan's content stops being a goal and
starts describing how things are, move it here and rewrite it in the present tense.

## Layout

| Path | Holds |
|---|---|
| `src/architecture/` | system design: `README.md` (skill intelligence), `seams.md`, `graphs/`, `models/` (runtime, deployment, sync), `pipelines/`, `retrieval/` |
| `src/architecture/legacy/` | superseded notes, kept for history |
| `src/dev/adrs/<topic>/NNNN-<slug>.md` | Architecture Decision Records (so far only `rust-wasm/`) |
| `src/dev/demo-datasets.md` | generated demo datasets: the `dataset_meta` marker, R2 key layout, push guards, operator flow |
| `src/migrations/` | guides for migrations still in progress |

## Conventions

- Design docs open with a `> Status:` line (for example `Design`, `Planning`) and, where they
  exist, the epics or beads they cover.
- **ADRs** are numbered per topic directory. Each opens with `# NNNN. Title`, then `Status`,
  `Date` and the ADRs or roadmap milestones it builds on. Once an ADR is accepted, never
  rewrite its decision: write a new ADR that supersedes it.
- `legacy/` is history. Don't update it; write a current document instead.
- A migration guide deletes itself when its migration is complete (see its `Self-destruct:`
  line).
- `src/talks/`, `src/prep/`, `src/blog/` and `src/analysis/` are gitignored
  (`docs/.gitignore`). They are private: never commit or cite them.
- Never create `docs/superpowers/`. Specs and plans go in `.agents/plans/`.
