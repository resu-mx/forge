# M6 — Page Overlay + Answer Bank + App Questions

**Status**: Approved
**Date**: 2026-04-20
**Version**: 0.1.6
**Beads**: 3bp.30 (overlay), 3bp.28 (answer bank), 3bp.26 (app questions)
**Depends on**: M4 (dropdown filling), M5a (parser wiring)

## Overview

Largest MVP phase. Three interlocking features:
1. **Page overlay** — shadow DOM side panel for reviewing/editing extracted fields before API submit
2. **Answer bank** — Forge DB-backed storage for reusable EEO/work-auth answers, managed via WebUI settings
3. **Workday app questions** — extend M4 field detection + fill to use answer bank for EEO/work-auth fields

## 1. Capture Flow (Revised)

### Current flow
Popup → background `captureActive` → inject content script → extract → background → API → popup shows result.

### New flow
1. **Trigger** (popup button, context menu, or injected button) → background injects content script
2. Content script extracts + runs `enrichWithParser()` → produces `ExtractedJob` with confidence scores
3. **Decision gate** evaluates confidence against configurable floors:
   - **All fields pass** → content script sends to background → API → **toast** ("Captured: SWE at Anthropic") auto-dismisses 3s
   - **Any field fails floor** → content script injects **side panel overlay** with editable fields, low-confidence fields highlighted
   - **Manual mode** (Shift+click capture, or config toggle) → always shows overlay regardless of confidence
4. User reviews/edits in overlay → clicks "Submit" → content script sends edited `ExtractedJob` to background → API → toast confirms
5. "Cancel" dismisses overlay, no API call

The popup's role shrinks: it triggers capture and shows health status. The overlay replaces popup-as-review-UI. All three entry points (popup, context menu, injected button) converge to the same content script flow.

### Message flow changes

**New background commands:**
- `jd.submitExtracted` — accepts a pre-edited `ExtractedJob` from overlay (or directly from quiet-mode extraction), skips re-extraction, runs validation → dedup → org resolution → API create (steps 4-7 of current `handleCaptureJob`)
- `answers.list` — proxy to `GET /api/profile/answers` (for Workday fill)

**Content script → background messaging:**
- Quiet path: content script sends `{ cmd: 'jd.submitExtracted', data: ExtractedJob }` to background
- Overlay path: user clicks Submit → same message with edited fields

## 2. Confidence Model

### Types

```ts
type ConfidenceTier = 'high' | 'medium' | 'low' | 'absent'

interface FieldConfidence {
  field: string           // e.g. 'title', 'salary_min', 'work_posture'
  tier: ConfidenceTier
  source: string          // 'chip', 'selector', 'parser-body', 'missing'
}

interface EnrichedExtraction {
  extracted: ExtractedJob
  confidence: FieldConfidence[]
}
```

### Tier assignment

Baked into extraction and enrichment — each extraction source assigns a tier:
- **high**: Dedicated DOM element with strong selector (LinkedIn `h1`, chip elements, `data-automation-id`)
- **medium**: Parser-derived from body text (regex/heuristic in free text)
- **low**: Inferred or ambiguous (e.g., location parsed but could be office name vs. city)
- **absent**: Field not found at all

### Field priority and default floors

| Field | Priority | Default Floor | Notes |
|-------|----------|---------------|-------|
| `title` | High | `high` | LinkedIn `h1` selector |
| `company` | High | `high` | LinkedIn company link |
| `salary_min`/`salary_max` | High | `medium` | Chip = high, parser body = medium |
| `work_posture` | Medium-High | `medium` | Chip or parser |
| `location` | Medium | `medium` | Chip = high, parser = medium |
| `company_url` | High | `high` | LinkedIn company href |
| `url` (post URL) | High | `high` | `window.location.href` |
| `apply_url` | Medium | `low` | May not exist (Easy Apply) |
| `description` (raw_text) | High | `high` | JD body text |
| `source_plugin` | High | `high` | Always present |
| Parsed requirements | Low | `absent` | Never triggers overlay |
| Parsed responsibilities | Low | `absent` | Never triggers overlay |
| Parsed preferred | Low | `absent` | Never triggers overlay |

### Decision gate

```ts
function shouldShowOverlay(
  confidence: FieldConfidence[],
  floors: Record<string, ConfidenceTier>,
  forceManual: boolean
): boolean
```

Tier ordering: `high > medium > low > absent`. If any field's actual tier is strictly below its configured floor → show overlay. `forceManual` bypasses all checks.

### Configurable modes

Stored in `chrome.storage.local` under `confidenceMode` key. Editable from popup settings gear (future) or extension storage direct.

- **User mode** (default): Floors as table above. Quiet when extraction is clean.
- **Debug mode**: All floors set to `high` — overlay shows on any parser-derived field.
- **Dev mode**: All floors set to `absent` — overlay never auto-triggers (manual only).

**Seam**: Mode system is defined and wired but UI for switching modes is a popup settings gear — can be stubbed as a simple dropdown in M6, polished in M7.

## 3. Side Panel Overlay

### Injection

Shadow DOM container attached to `document.body`:
```
position: fixed; right: 0; top: 0; height: 100vh; width: 360px; z-index: 2147483647;
```

Content script creates the shadow host, attaches shadow root (`mode: 'closed'`), injects self-contained HTML + CSS. No host CSS leakage.

### Layout (top to bottom)

1. **Header**: "Forge -- Review Extraction" + close (X) button
2. **Confidence summary**: Badge count — "3 fields need review"
3. **Field list**: Each field as a labeled row:
   - High-confidence fields: value displayed with subtle green checkmark, editable on click
   - Medium-confidence fields: amber left border, editable input shown
   - Low/absent fields: red left border, empty input with placeholder
   - All fields are always editable regardless of confidence
4. **Parsed sections** (collapsed by default): Accordion for requirements/responsibilities/preferred — read-only preview from parser output
5. **Footer**: "Cancel" (ghost button) + "Submit to Forge" (primary button)

### Styling

Dark theme matching popup aesthetic. Fully self-contained in shadow DOM:
- Background: `#1a1a2e`
- Text: `#e0e0e0`
- Accent: `#6366f1` (primary), `#4ade80` (high confidence), `#fbbf24` (medium), `#f87171` (low/absent)
- Inputs: dark background with subtle border, focus ring on edit

### Toast notification

For quiet-mode captures. Small fixed-position element, bottom-right corner:
```
position: fixed; bottom: 20px; right: 20px; z-index: 2147483647;
```
Shows "Captured: [title] at [company]" with checkmark icon. Auto-dismisses after 3s with fade-out. Also shadow DOM to avoid CSS collision.

## 4. Answer Bank

### Database

Migration `050_answer_bank.sql`:
```sql
CREATE TABLE answer_bank (
  id TEXT PRIMARY KEY DEFAULT (forge_uuid()),
  field_kind TEXT NOT NULL UNIQUE,
  label TEXT NOT NULL,
  value TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

`field_kind` values from `FieldKind` type: `work_auth.us`, `work_auth.sponsorship`, `eeo.gender`, `eeo.race`, `eeo.veteran`, `eeo.disability`.

### API endpoints

Mounted under profile routes (`/api/profile/answers`):

| Method | Path | Body | Response |
|--------|------|------|----------|
| `GET` | `/api/profile/answers` | — | `{ data: AnswerBankEntry[] }` |
| `PUT` | `/api/profile/answers` | `{ field_kind, label, value }` | `{ data: AnswerBankEntry }` |
| `DELETE` | `/api/profile/answers/:field_kind` | — | 204 |

PUT is upsert — creates if `field_kind` doesn't exist, updates if it does.

### Service

`AnswerBankService` with methods:
- `list(): Result<AnswerBankEntry[]>`
- `upsert(input: { field_kind: string; label: string; value: string }): Result<AnswerBankEntry>`
- `delete(fieldKind: string): Result<void>`

### SDK resource

`AnswerBankResource`:
- `list(): Promise<Result<AnswerBankEntry[]>>`
- `upsert(data): Promise<Result<AnswerBankEntry>>`
- `delete(fieldKind): Promise<Result<void>>`

Accessed via `client.profile.answers` (nested under profile).

## 5. WebUI — Settings Pages

Answer bank management under Forge settings:

### `/settings/work-auth`
Work authorization fields:
- **Authorized to work in US**: Yes / No radio
- **Require sponsorship**: Yes / No radio

### `/settings/eeo`
EEO voluntary disclosures:
- **Gender**: Select (Male, Female, Non-binary, Prefer not to say, Decline to self-identify)
- **Race/Ethnicity**: Select (standard EEO categories)
- **Veteran status**: Select (Protected Veteran, Not a Veteran, Prefer not to say)
- **Disability status**: Select (Yes, No, Prefer not to say)

Both pages: load from `GET /api/profile/answers`, save via `PUT /api/profile/answers` on change. Uses existing UI shared components (PageWrapper, PageHeader, etc.).

## 6. Workday App Questions (3bp.26)

### Extended field detection

New entries in `FIELD_NAME_MAP` for Workday's Application Questions and Voluntary Disclosures:

```ts
// Application Questions
'workAuthorizationStatus': 'work_auth.us',
'sponsorshipRequired': 'work_auth.sponsorship',

// Voluntary Disclosures (EEO)
'gender': 'eeo.gender',
'race': 'eeo.race',
'veteranStatus': 'eeo.veteran',
'disabilityStatus': 'eeo.disability',
```

**Seam**: Workday field names vary by employer configuration. The map above covers the most common `data-automation-id` patterns. Additional mappings added as encountered — the architecture supports arbitrary `FieldKind` → `data-automation-id` mappings.

### Fill flow changes

1. `handleProfileFill` already loads profile → builds field map from `buildProfileFieldMap()`
2. **New**: Also fetch `client.profile.answers.list()` (answer bank entries)
3. **Merge**: Answer bank values keyed by `field_kind` are added to the field map. Answer bank entries take precedence for EEO/work-auth kinds (profile map doesn't have these).
4. Content script fills as before — M4's `fillField` handles all field types (text, radio, select, custom-dropdown)

## 7. File Changes

### New files

| File | Purpose |
|------|---------|
| `packages/core/src/db/migrations/050_answer_bank.sql` | Answer bank table |
| `packages/core/src/services/answer-bank-service.ts` | CRUD service |
| `packages/core/src/routes/answer-bank.ts` | HTTP routes |
| `packages/sdk/src/resources/answer-bank.ts` | SDK resource |
| `packages/webui/src/routes/settings/work-auth/+page.svelte` | Work auth settings page |
| `packages/webui/src/routes/settings/eeo/+page.svelte` | EEO settings page |
| `packages/extension/src/lib/confidence.ts` | Tier model, decision gate, mode config |
| `packages/extension/src/content/overlay.ts` | Shadow DOM side panel |
| `packages/extension/src/content/toast.ts` | Toast notification |

### Modified files

| File | Changes |
|------|---------|
| `packages/extension/src/plugin/types.ts` | Add `ConfidenceTier`, `FieldConfidence`, `EnrichedExtraction` |
| `packages/extension/src/lib/enrich-extraction.ts` | Return confidence scores alongside enriched fields |
| `packages/extension/src/content/linkedin.ts` | After extraction: decision gate → overlay or toast |
| `packages/extension/src/background/handlers/capture.ts` | Add `handleSubmitExtracted` for pre-edited data from overlay |
| `packages/extension/src/background/handlers/autofill.ts` | Merge answer bank into fill values |
| `packages/extension/src/background/index.ts` | New commands: `jd.submitExtracted`, `answers.list` |
| `packages/extension/src/lib/messaging.ts` | New Command variants |
| `packages/extension/src/plugin/plugins/workday.ts` | EEO/work-auth field detection in `FIELD_NAME_MAP` |
| `packages/extension/src/content/workday.ts` | Handle answer-bank-augmented fill |
| `packages/extension/src/popup/Popup.svelte` | Shift+click for manual overlay |
| `packages/extension/vite.config.ts` | Add overlay + toast as content script entry points |
| `packages/core/src/routes/server.ts` | Mount answer bank routes |
| `packages/core/src/services/index.ts` | Register AnswerBankService |
| `packages/sdk/src/client.ts` | Wire AnswerBankResource |
| `packages/sdk/src/types.ts` | AnswerBankEntry type |
| `packages/sdk/src/index.ts` | Export answer bank types |
| `manifest.json` | Version bump to 0.1.6 |
| `manifest.firefox.json` | Version bump to 0.1.6 |

## 8. Success Criteria

- [ ] Overlay renders on capture when any field fails confidence floor
- [ ] Edited values in overlay persist to Forge on submit
- [ ] Quiet mode: toast notification on high-confidence captures, no overlay
- [ ] Manual mode: Shift+click always shows overlay
- [ ] Confidence mode configurable (User/Debug/Dev)
- [ ] Answer bank CRUD via API + WebUI settings pages
- [ ] Workday EEO/work-auth fields detected and filled from answer bank
- [ ] All entry points (popup, context menu, injected button) use new flow
- [ ] Shadow DOM overlay has no CSS leakage on LinkedIn/Workday
- [ ] Extension builds and loads in both Chrome and Firefox/Zen
- [ ] Version bumped to 0.1.6

## 9. Identified Seams (Stubbed for Now)

- **Confidence mode UI**: Popup settings gear for switching User/Debug/Dev modes. M6 stores in `chrome.storage.local`, but no UI toggle — changed via dev tools or extension options page stub.
- **Per-field floor customization**: Architecture supports per-field overrides, but M6 only exposes preset modes. Granular config deferred.
- **Overlay → modal hybrid**: Side panel for M6, blended side-panel + centered-modal approach deferred.
- **Workday field name variations**: `FIELD_NAME_MAP` covers common patterns. Employer-specific variations require discovery and addition to the map.
- **Answer bank field expansion**: Architecture supports arbitrary `field_kind` values beyond EEO/work-auth. New fields added by inserting rows.
