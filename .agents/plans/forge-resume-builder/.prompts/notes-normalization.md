# Notes Normalization Session Prompt

Paste this into a new Claude Code session to pick up the notes normalization work.

---

## Context

Check memory for `project_session_2026_04_15.md` for the most recent session context.

I want to work on **notes normalization** (bead `job-hunting-sep`). This moves inline `notes TEXT` columns from 8 entity tables into the existing `user_notes` + `note_references` tables.

## Beads

The epic has 5 child tasks — check with `bd show job-hunting-sep` and `bd children job-hunting-sep` for current state.

- `sep.1` — Migration: move inline notes to Notes table + note_references
- `sep.2` — Update services to read/write notes via note_references
- `sep.3` — Add MCP note tools (create, search, link, unlink)
- `sep.4` — Update WebUI to use shared notes component
- `sep.5` — Tests

## What Exists Already

- `user_notes` table + `note_references` junction table (migration 002)
- `NoteService` in `packages/core/src/services/note-service.ts`
- `note_references` has CHECK constraint on entity_type
- Existing note routes + SDK resource

## Tables With Inline Notes (8)

1. `sources.notes`
2. `bullets.notes`
3. `perspectives.notes` (if present — verify)
4. `resumes.notes`
5. `resume_entries.notes`
6. `skills.notes`
7. `organizations.notes`
8. `job_descriptions.notes`

## Approach

Start with `sep.1` (migration) in a worktree. The migration should:
1. For each table with inline notes, INSERT into `user_notes` from non-null notes
2. Create `note_references` rows linking back to the source entity
3. Drop the `notes` columns from all 8 tables

Use a worktree (`git worktree add .claude/worktrees/notes-normalization -b feat/notes-normalization`).

## Key Files to Read First

- `packages/core/src/services/note-service.ts` — existing NoteService
- `packages/core/src/db/migrations/002_schema_evolution.sql` — user_notes + note_references schema
- `packages/core/src/types/index.ts` — filter types that reference notes
- Feature spec: `.claude/plans/forge-resume-builder/.feats/data-model/consolidate_inline_notes.md`

## Dev Commands

```bash
just dev          # Start API + MCP + WebUI
just test         # Run all tests
just test-core    # Core tests only
just migrate      # Run DB migrations
```
