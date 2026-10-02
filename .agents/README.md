# `.agents/`: agent context

This is the vendor-neutral home for the context coding agents use in Forge: the index every
agent reads first, shared context documents, and working plans. Guidance about one part of
the code lives in an `AGENTS.md` next to that code, and the index here points to each one.

## Layout

```text
AGENTS.md                     root instructions, loaded at the start of every session
.agents/
├── AGENTS.md                 the index: imported by the root file, re-read after compaction
├── README.md                 this file
├── context/                  shared, task-triggered context (e.g. beads-taxonomy.md)
└── plans/                    working plans, specs and prompts (layout in the index)
packages/AGENTS.md            one per major directory, next to the code it describes
packages/webui/AGENTS.md
packages/extension/AGENTS.md
crates/AGENTS.md
docs/AGENTS.md
.docker/AGENTS.md
.github/AGENTS.md
.claude/                      Claude Code-only config: path-scoped rules, skills, hooks
```

## How each agent loads it

| Agent | Root `AGENTS.md` | `.agents/AGENTS.md` | A directory's `AGENTS.md` |
|---|---|---|---|
| Claude Code ≥ 2.1.277 | read natively, because the repo has no `CLAUDE.md` | expanded from the root's `@.agents/AGENTS.md` import | loaded automatically when Claude reads a file in that directory |
| Codex | read from the repo root down to the working directory | via the root's directive | those on the path to the working directory; the rest via the index |
| Others | per the tool's own docs | via the root's directive | via the index |

Claude Code never reads anything under `.agents/` by itself. That is why the root file both
imports the index and tells every agent to read it at the start of each conversation and
after each compaction.

**Do not add a `CLAUDE.md`**, at the root or in any directory. Claude Code reads `AGENTS.md`
only when no `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md` exists in or above the
working directory. One such file switches the whole tree off for Claude. A personal
`CLAUDE.local.md` does the same, unless you set `/config` → Project instructions to
`claude-md-and-agents-md`. Claude-specific guidance belongs in `.claude/rules/` (with `paths:`
frontmatter) or `.claude/skills/`.

## Adding context

1. **Guidance for a directory:** create `<dir>/AGENTS.md` covering what the directory owns,
   how to build and test it, and its invariants and gotchas. Then add a row to the index.
2. **Shared context that a task triggers rather than a path:** add
   `.agents/context/<topic>.md` and an index row.
3. **Claude-only guidance tied to files:** add `.claude/rules/<topic>.md` with `paths:`
   frontmatter. Claude Code ignores `globs:`, which loads the rule in every session instead.
4. Keep each fact in one place and link to it rather than copying. Root files are read every
   session, so keep them short.

## Troubleshooting

- In Claude Code, `/memory` lists the instruction files that loaded. Expect `AGENTS.md`. After
  Claude reads a file under `crates/`, expect `crates/AGENTS.md` as well.
- If `AGENTS.md` is missing from that list:
  - look for a `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md` on the path
  - confirm `claude --version` is at least 2.1.277
  - confirm `/config` → Project instructions is not set to `claude-md`
- A worktree under `.claude/worktrees/` sits inside the canonical checkout, so the
  checkout's root instruction files are on its path too.
