# Work tracking: milestones, issues and beads

Read this before you create a GitHub milestone or issue, or generate beads from an issue.

Forge tracks work in three layers. Each has one audience, so each says different things.

| Layer | Lives in | Audience | Holds |
|---|---|---|---|
| Milestone | GitHub (`resu-mx/forge`) | humans | an effort or objective |
| Issue | GitHub | humans | the high-level what and why |
| Bead | `bd` | agents | the low-level, tightly scoped how |

## Milestones

- One milestone per effort or objective, for example `RS<->TS Feature Parity` or
  `Live breakage recovery`. Its description states the objective and any milestones it depends on.
- A milestone groups **parent issues and childless issues**. A child issue carries its parent's
  milestone.

## Issues (for humans)

- Three shapes: **parent** (has sub-issues), **child** (a GitHub sub-issue; exactly one parent)
  and **childless**.
- Use GitHub's own relationships: sub-issues for hierarchy, "blocked by" for dependencies. Don't
  keep hand-written task lists of children in the body.
- Issue type: `Bug` for broken behaviour, `Feature` for new or parity capability, `Task` for
  chores, decisions and docs.
- Keep it high level. Body sections, in this order:

  | Section | Holds |
  |---|---|
  | Motivation | why this matters now |
  | Goals | what changes for the user or the system |
  | Description | **Ideal** vs **Minimal**. A bug also gives Current behaviour, Steps to reproduce and Expected behaviour |
  | Spec | the contract or behaviour, at the level a reviewer checks |
  | Constraints | what must stay true (both runtimes, wasm32, budgets, compatibility) |
  | Examples | rough pseudocode |
  | Acceptance criteria | user / behavioural / E2E, as Given/When/Then checkboxes |
  | Refs | files, ADRs, docs, related issues and beads, links |

- The repository is public. Never put secrets, tokens, local paths or private-repo internals in an
  issue.

## Beads (for agents)

- **Where they live.** An embedded Dolt store in `.beads/embeddeddolt/` (gitignored), synced to
  the Dolt remote in this GitHub repository (`refs/dolt/*`, set as `sync.remote` in
  `.beads/config.toml`). Run `bd dolt pull` before you create beads and `bd dolt push` after. Not
  the shared Dolt server: a shell that exports `BEADS_DOLT_SERVER_*` overrides the repo config and
  silently points `bd` at it, so unset those first (`env | grep ^BEADS_DOLT`). `bd` run in a
  worktree uses the canonical checkout's `.beads/`.
- Beads are generated from an issue: a parent, a child or a childless one. **Issues and beads are
  1:N.** Hierarchy inside `bd` (`--parent`, `--deps`) is independent of GitHub's.
- Link each bead to its source issue with the label `gh:<number>` and a
  `Source: resu-mx/forge#<number>` line in the description. Find them with `bd list -l gh:<number>`.
- Don't set `--external-ref gh-<number>`, and don't run `bd github sync`, `push` or `pull`. They
  assume one bead per issue and would mirror agent-level beads into GitHub.
- Beads are detailed and tightly scoped:

  | `bd create` field | Holds |
  |---|---|
  | description | Spec, requirements (`REQ-1`, `REQ-2`, …), Constraints, Goals, Motivation, Refs |
  | `--design` | Examples: an 80% solution in the target language |
  | `--acceptance` | Definition of done, and acceptance criteria at unit / functional / integration level |

- Label each bead with exactly one category from `beads-taxonomy.md` (`cat:<category>`).

## Closing the loop

- A PR that completes an issue says `Closes #<number>` in its body. Commits carry the bead ID in a
  `Refs:` footer, never in the subject.
- An issue closes when all of its beads are closed. A parent closes when all of its children are
  closed.
