# Forge Browser Extension — MVP Session Prompt

Copy everything below into a new Claude Code session.

---

Forge browser extension — MVP phase. The prototype series (P1-P7) is
complete. Extension is at v0.0.9 with 58 tests passing.

## Load context first

1. Read memory: `project_forge_extension_p3_p6_2026_04_14.md` — has full
   epic state (12 closed, 11 open beads), key discoveries (shared chunk
   constraint, React _valueTracker fix), and MVP scope from SPEC §9.

2. Read SPEC §9 (MVP section): `.claude/plans/forge-resume-browser-extension/SPEC.md`
   — defines MVP deliverables beyond prototype.

3. Verify state:
   - `bd list --label forge-extension --pretty` — should show 12 closed, 11 open
   - `git log --oneline -3 main` — P7 merge should be recent
   - Extension version in `packages/extension/manifest.json` should be `0.0.9`

## What prototype proved

- **P1-P3**: Extension can extract JDs from LinkedIn and read/write Forge API
- **P4-P5**: Full capture flow with URL dedup and org resolution
- **P6-P7**: Workday form detection and profile-based autofill (text inputs)

## What MVP needs to add (from SPEC §9)

- [ ] Org resolve/create with disambiguation UX (multiple match handling)
- [ ] Page overlay with review-before-submit (editable extracted fields)
- [ ] Workday plugin — extraction + full autofill (dropdowns, not just text)
- [ ] Firefox support (WebExtensions polyfill, manifest adaptations)
- [ ] Application answer bank (stored EEO, work auth, sponsorship answers)
- [ ] Forge-hosted config migration (`GET /api/extension/config`)
- [ ] Forge server logging endpoint (`POST /api/extension/log`)

## Open beads to triage for MVP

Priority P2 (should be MVP):
- `3bp.19` — JD location → Org Campus linkage
- `3bp.22` — Workday dropdown filling (country code, state, custom selects)
- `3bp.23` — Workday Application Questions + Voluntary Disclosures pages

Priority P3 (could be MVP or post-MVP):
- `3bp.13` — LinkedIn salary chip truncation
- `3bp.14` — Apply link extraction
- `3bp.15` — Salary from JD body text
- `3bp.16` — Org LinkedIn URL capture
- `3bp.17` — Work posture field
- `3bp.18` — Multiple locations
- `3bp.20` — Session cache
- `3bp.21` — Context menu "Capture to Forge"

## Hard rules (carried from prototype)

- All work in worktrees under `.claude/worktrees/` — never on main
- Every phase must produce a buildable, dev-installable extension
- Use programmatic injection — no `content_scripts` in manifest
- Background worker must NOT import plugin modules (shared chunk bug)
- Rebuild `packages/extension/dist/` on main after every merge
- `bd close <id>` with commit SHA on phase completion
- Subagent-driven development for implementation

## Start here

1. Load context (steps above)
2. Brainstorm MVP phasing — which open beads group into which MVP phases?
3. Prioritize: what delivers the most user value first?
4. Draft the first MVP phase plan
