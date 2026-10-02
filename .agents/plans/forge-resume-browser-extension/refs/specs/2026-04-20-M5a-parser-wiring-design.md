# M5a — Parser Wiring + Core Persist Design

**Status**: Approved
**Date**: 2026-04-20
**Scope**: M5a (extension wiring + DB columns + MCP ingest). M5b (campus resolution) deferred.
**Beads**: 3bp.21.5, 3bp.21.6, 3bp.15, 3bp.17, 3bp.18
**Version**: 0.1.5

## Overview

Wire the M3 parser (`packages/core/src/parser/`) into both consumers — the browser extension capture flow and the Forge MCP ingest tool — so that parsed sections, salary, locations, and work posture are auto-populated and persisted on every JD.

## Extension Data Flow

### Current
```
linkedin.ts: extractJD() → raw fields → background
capture.ts: validate → resolveOrg → SDK create (raw fields only)
```

### M5a
```
linkedin.ts: extractJD() → parseJobDescription(description) → enriched response
    ↓ (message to background)
capture.ts: validate → resolveOrg → SDK create (raw + parsed fields)
    ↓
DB: job_descriptions row with parsed_sections, salary_min/max, work_posture, parsed_locations
```

### Why parser runs in content script (not background)
- Avoids importing `@forge/core` in background worker, preventing shared Vite chunks
- Parser is ~250 LOC of pure functions — Vite inlines into content script IIFE bundle
- Parsed sections immediately available in content script context for M6 overlay (no messaging round-trip)

### Enrichment logic
After plugin extracts raw fields, content script calls `parseJobDescription(description)`:
- `salary_min`/`salary_max` from `parsed.salary` (overrides chip-based salary when parser finds structured data)
- `salary_period` from `parsed.salary.period`
- `work_posture` from `parsed.workPosture`
- `parsed_locations` from `parsed.locations`
- `parsed_sections` from `parsed.sections` (JSON-serialized `ClassifiedSection[]`)

Background capture handler (`handleCaptureJob`) maps these directly to SDK create params. No new logic beyond forwarding.

## Core Schema Changes

### Existing columns (migration 027)
- `salary_min` INTEGER, nullable
- `salary_max` INTEGER, nullable

Parser auto-populates these. No schema change needed.

### New migration (additive, no data loss)

| Column | Type | Description |
|--------|------|-------------|
| `parsed_sections` | TEXT | JSON `ClassifiedSection[]` from L1+L2 |
| `work_posture` | TEXT | `'remote'\|'hybrid'\|'on-site'` or NULL |
| `parsed_locations` | TEXT | JSON `string[]` (e.g., `["San Francisco, CA"]`) |
| `salary_period` | TEXT | `'annual'\|'hourly'\|'unknown'` or NULL |

No new indexes — columns are for display/overlay, not query filtering.

### Backfill
Same migration re-parses existing JDs via `parseJobDescription(raw_text)` and populates all parser-derived columns including `salary_min`/`salary_max`. Idempotent — parser is deterministic.

## SDK + API Changes

### SDK types
`CreateJobDescription` and `UpdateJobDescription` gain optional fields:
- `work_posture?: string`
- `parsed_locations?: string` (JSON string)
- `parsed_sections?: string` (JSON string)
- `salary_period?: string`

### JD Service
`create()` and `update()` pass new fields through to SQL. No business logic changes.

### MCP Ingest (`forge_ingest_job_description`)
- Calls `parseJobDescription(raw_text)` after receiving input
- Auto-populates: `salary_min`, `salary_max`, `salary_period`, `work_posture`, `parsed_locations`, `parsed_sections`
- User-supplied `salary_min`/`salary_max` override parser values (explicit > inferred)
- No new tool parameters — parser runs transparently server-side
- Returned `JobDescriptionWithOrg` includes parsed fields automatically

## Testing Strategy

### Extension (~4 tests)
- Content script calls parser, returns enriched payload
- Capture handler forwards parsed fields to SDK create
- Build output test confirms no leaked `import` statements in IIFE bundles

### Core (~3 tests)
- Migration backfill populates parsed fields for existing JDs
- JD service create/update round-trips new fields

### MCP (~2 tests)
- Ingest tool auto-populates parser fields
- User-supplied salary overrides parser salary

No new parser tests — M3's 58 tests cover the parser itself.

## Deferred to M5b
- **3bp.19**: JD location → Org Campus linkage (campus resolution from parsed locations)

## Architecture Constraints
- Content scripts: IIFE bundles, Vite inlines all imports (including `@forge/core/parser`)
- Background worker: must NOT import `@forge/core` parser directly (shared chunk risk)
- Parser: pure functions, zero dependencies, safe to inline anywhere
