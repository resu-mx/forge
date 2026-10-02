# CLI: `forge review` UX Mockup

## Entry Screen

```
╭─ Forge Review Queue ─────────────────────────────────╮
│                                                       │
│  Pending:  5 bullets  ·  3 perspectives               │
│                                                       │
│  Starting with bullets...                             │
│                                                       │
╰───────────────────────────────────────────────────────╯
```

## Bullet Review

```
── Bullet Review [1/5] ─────────────────────────────────

Source: "Cloud Forensics Platform Migration"
  (Raytheon · 2023-01-15 to 2023-07-15)

Snapshot matches current source: ✓

Content:
  "Led 4-engineer team migrating cloud forensics platform
   from ELK to AWS OpenSearch over 6 months"

Technologies: ELK, AWS OpenSearch
Metrics: 4 engineers, 6 months

────────────────────────────────────────────────────────
  [a]pprove   [r]eject   [s]kip   [q]uit
> _
```

## Rejection Flow

```
> r
Rejection reason: Overstates scope — was a contributor, not the lead
✗ Rejected.

── Bullet Review [2/5] ─────────────────────────────────
...
```

## Perspective Review

```
── Perspective Review [1/3] ────────────────────────────

Bullet: "Led 4-engineer team migrating cloud forensics
         platform from ELK to AWS OpenSearch over 6 months"

Snapshot matches current bullet: ✓

Archetype: agentic-ai
Domain: ai_ml
Framing: accomplishment

Content:
  "Led cloud platform migration enabling ML-based log
   analysis pipeline on AWS OpenSearch"

────────────────────────────────────────────────────────
  [a]pprove   [r]eject   [s]kip   [q]uit
> _
```

## Snapshot Drift Warning

```
── Bullet Review [3/5] ─────────────────────────────────

Source: "Cloud Forensics Platform Migration"

⚠ Snapshot DIFFERS from current source:

  Snapshot (at derivation):
    "Led a team of 4 engineers to migrate..."

  Current:
    "Led a team of 5 engineers to migrate..."
                      ^ changed

Content:
  "Led 4-engineer team migrating..."

────────────────────────────────────────────────────────
  [a]pprove   [r]eject   [s]kip   [q]uit
> _
```

## Completion

```
── Review Complete ─────────────────────────────────────

  Approved:  4 bullets, 2 perspectives
  Rejected:  1 bullet, 1 perspective
  Skipped:   0

  Remaining: 0 items pending review
```
