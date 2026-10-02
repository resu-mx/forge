# Phase 6: CLI

**Goal:** Implement the `forge` CLI tool as an SDK consumer.

**Non-Goals:** No UI beyond terminal output. No direct database access.

**Depends on:** Phase 5 (SDK)
**Can parallelize with:** Phase 7 (WebUI) — both are SDK consumers

---

## Task 6.1: CLI Framework Setup

**File:** `packages/cli/src/index.ts`

**Steps:**
1. Choose CLI framework: `citty` (lightweight, TypeScript-native) or `commander`
2. Set up root command `forge` with subcommands
3. Configure SDK client from environment: `FORGE_API_URL` (default: `http://localhost:3000`)
4. Add global `--json` flag for machine-readable output
5. Add connection check: if SDK returns NETWORK_ERROR, print helpful message and exit 1

**Acceptance Criteria:**
- [ ] `forge --help` shows all subcommands
- [ ] `forge --version` shows package version
- [ ] `--json` flag available on all commands
- [ ] Connection failure shows: "Cannot connect to Forge server at {url}. Start it with 'just dev'."
- [ ] Exit codes: 0 success, 1 error, 2 validation error

**Testing:**
- Smoke: `forge --help` exits 0
- Unit: Connection failure message

---

## Task 6.2: Source Commands

**Commands:**
```
forge source add       # interactive prompts for title, description, employer, dates
forge source list      # table output, --json for JSON
forge source show <id> # full detail view
forge source edit <id> # opens $EDITOR or interactive prompts
forge source delete <id>
forge source derive-bullets <id>  # triggers derivation, shows results
```

**Output format (table):**
```
ID                                    Title                          Status    Bullets
────────────────────────────────────  ─────────────────────────────  ────────  ───────
550e8400-e29b-41d4-a716-44665544...  Cloud Forensics Migration      approved  4
661f9511-f30c-42e5-b827-55776655...  SIEM Pipeline Redesign         draft     0
```

**derive-bullets output:**
```
Deriving bullets from "Cloud Forensics Migration"... (up to 60s)

✓ 4 bullets created (status: pending_review)

  1. Led 4-engineer team migrating cloud forensics platform...
     Technologies: ELK, AWS OpenSearch

  2. Reduced mean incident response time by 40%...

  ...

Run 'forge review' to approve or reject these bullets.
```

**Acceptance Criteria:**
- [ ] All 6 source commands implemented
- [ ] `add` prompts for required fields interactively
- [ ] `list` shows formatted table, `--json` shows JSON array
- [ ] `derive-bullets` shows spinner/progress during AI call
- [ ] `derive-bullets` handles timeout and conflict errors with clear messages

**Testing:**
- Integration: Create source via CLI, verify via `list`
- Integration: `derive-bullets` with mocked API
- Unit: Output formatting (table and JSON modes)

---

## Task 6.3: Bullet Commands

**Commands:**
```
forge bullet list [--source <id>] [--status <status>] [--technology <name>]
forge bullet show <id>
forge bullet approve <id>
forge bullet reject <id> --reason "..."
forge bullet reopen <id>
forge bullet delete <id>
forge bullet derive-perspectives <id> --archetype <name> --domain <name> --framing <type>
```

**Acceptance Criteria:**
- [ ] All 7 bullet commands implemented
- [ ] `reject` requires `--reason` flag, errors without it
- [ ] `derive-perspectives` requires archetype, domain, framing flags
- [ ] Filters work on `list` command

**Testing:**
- Integration: Full bullet lifecycle via CLI
- Unit: Missing `--reason` on reject → error message

---

## Task 6.4: Perspective Commands

**Commands:**
```
forge perspective list [--bullet <id>] [--archetype <name>] [--domain <name>]
forge perspective show <id>    # shows full chain
forge perspective approve <id>
forge perspective reject <id> --reason "..."
forge perspective reopen <id>
forge perspective delete <id>
```

**`show` output includes chain:**
```
Perspective: Led cloud platform migration enabling ML-based...
  Archetype: agentic-ai | Domain: ai_ml | Framing: accomplishment
  Status: pending_review

  Chain:
    └─ Bullet: Led 4-engineer team migrating cloud forensics...
       Snapshot match: ✓
       └─ Source: Cloud Forensics Platform Migration
          Snapshot match: ✓
```

**Acceptance Criteria:**
- [ ] All 6 perspective commands implemented
- [ ] `show` displays full chain with snapshot match indicators

**Testing:**
- Integration: Full perspective lifecycle via CLI

---

## Task 6.5: Resume Commands

**Commands:**
```
forge resume create           # interactive prompts
forge resume list
forge resume show <id>        # shows sections with perspectives
forge resume delete <id>
forge resume add-perspective <resume-id> <perspective-id> --section <name> --position <n>
forge resume remove-perspective <resume-id> <perspective-id>
forge resume reorder <resume-id>   # interactive section/position reorder
forge resume gaps <id>        # shows gap analysis
forge resume export <id>      # 501 message
```

**gaps output:**
```
Gap Analysis: AI Engineer - Anthropic (archetype: agentic-ai)

  Perspectives included: 8
  Skills covered: python, aws, kubernetes, terraform
  Domains: software_engineering, devops

  ⚠ Missing domain coverage:
    • ai_ml — 2 bullets available for derivation
    • leadership — no bullets available

  ⚠ Unused bullets (no perspective for agentic-ai):
    • "Built infrastructure automation with Terraform..." (source: Cloud Forensics)
```

**Acceptance Criteria:**
- [ ] All 8 resume commands implemented
- [ ] `gaps` displays structured gap report
- [ ] `export` prints 501 message with `just export-resume` instructions

**Testing:**
- Integration: Resume assembly workflow via CLI
- Integration: Gap analysis output formatting

---

## Task 6.6: Interactive Review (`forge review`)

**Reference:** `refs/uiux/mockups/forge-review-cli.md`

**Implementation:**
1. Call `forge.review.pending()` to get queue
2. Walk through items one at a time
3. For each item: display content, context, snapshot match
4. Prompt: [a]pprove / [r]eject / [s]kip / [q]uit
5. On reject: prompt for reason
6. Show summary at end

**Snapshot diff display:**
When `source_content_snapshot` differs from `source.description` (or `bullet_content_snapshot` from `bullet.content`), show the divergence inline. Implementation: simple string comparison — if not equal, show both with a `⚠ Snapshot DIFFERS` header. Full diff algorithm (word-level or line-level) is a post-MVP enhancement. For MVP, show both strings and let the human spot the difference.

**Acceptance Criteria:**
- [ ] Walks through all pending bullets then perspectives
- [ ] Shows source/bullet context for each item
- [ ] Shows snapshot match indicator (✓ or ⚠ with both strings)
- [ ] Reject prompts for reason
- [ ] Skip moves to next item
- [ ] Quit shows summary of actions taken
- [ ] `--json` mode outputs queue as JSON (non-interactive)

**Fallback Strategy:** If interactive terminal input is problematic in Bun, use a simpler confirm-style prompt per item rather than single-keypress navigation.

**Testing:**
- Integration: Create pending items, run review, verify status changes
- Unit: Output formatting
- Unit: Summary counts

---

---

## Parallelization

All command groups can be developed in parallel after Task 6.1:

```
Task 6.1 (scaffold) ──┬──► Task 6.2 (sources)
                       ├──► Task 6.3 (bullets)
                       ├──► Task 6.4 (perspectives)
                       ├──► Task 6.5 (resumes)
                       └──► Task 6.6 (review)
```

## Documentation

- `docs/src/cli/commands.md` — command reference with examples
- `docs/src/cli/usage.md` — getting started, common workflows
