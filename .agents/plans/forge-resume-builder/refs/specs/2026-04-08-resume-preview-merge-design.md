# Resume Editor/Preview Merge + PDF Caching

**Date:** 2026-04-08
**Status:** Approved
**Goal:** Merge Preview and Source tabs into a unified Editor/Preview model with format toggle, server-side PDF caching, and auto-rendering.

## Current State

Three tabs: Editor (DnD), Preview (PDF with manual generate button), Source (LaTeX/Markdown with sub-modes View/Edit). Problems:

1. Preview shows "Generate PDF" button instead of auto-rendering
2. PDF generates twice (debounce effect + button click race)
3. Document outline panel always open in rendered PDF
4. Source tab PDF preview is a tiny sliver (bad CSS)
5. Every tab switch triggers a full tectonic recompile (no caching)
6. Two separate components (`PdfView`, `SourceView`) duplicate PDF rendering logic

## Architecture

### Tab Structure

```
├── Editor
│   └── format
│       ├── default (DnD)
│       ├── latex (CodeMirror, changes require save + linter pass)
│       └── markdown (CodeMirror, changes require save + linter pass)
└── Preview
    ├── <RotationIcon> (regenerate / invalidate cache)
    ├── <DownloadIcon> (download PDF)
    └── format
        ├── default (PDF rendered from IR)
        └── markdown (GFM rendered from IR)
```

### Format Toggle

Format is a global state that persists across Editor/Preview tab switches:

| Format | Editor | Preview |
|--------|--------|---------|
| `default` | `<DragNDropView>` | PDF iframe (from server cache) |
| `latex` | CodeMirror (LaTeX) | PDF iframe (from server cache) |
| `markdown` | CodeMirror (Markdown) | Rendered GFM (client-side) |

Switching from Editor to Preview does NOT change the format. If you're editing Markdown and click Preview, you see rendered Markdown, not PDF.

### Data Flow

All previews render from committed IR state. Never from the editor buffer.

```
DnD edit
  → immediate IR mutation
  → server cache invalidated
  → Preview shows PDF from IR

LaTeX edit
  → manual save
  → linter validation pass
  → latex_override updated in IR
  → server cache invalidated
  → Preview shows PDF compiled from IR (with override)

Markdown edit
  → manual save
  → linter validation pass
  → markdown_override updated in IR
  → Preview shows GFM rendered from IR (with override)

Regenerate button (Preview tab)
  → invalidate server cache
  → recompile PDF from current IR
  → NOT from editor buffer
```

### Save Semantics by Format

| Format | Save behavior |
|--------|--------------|
| DnD (default) | Edits are live — drag/drop and entry changes mutate IR immediately |
| LaTeX | Explicit save required. Must pass LaTeX linter before save. Saves to `latex_override` field. |
| Markdown | Explicit save required. Must pass linter before save. Saves to `markdown_override` field. |

LaTeX and Markdown editors should show dirty/unsaved state clearly (e.g., dot in tab, "Unsaved changes" indicator). Navigating away from a dirty editor should warn.

## Server-Side PDF Cache

### Cache Strategy

Hash the LaTeX content before tectonic compilation. If a cached PDF exists for that hash, return it without recompiling.

**Location:** `/tmp/forge-pdf-cache/`

**Key:** SHA-256 of the final LaTeX string (after IR compile + template render + override application)

**Flow:**
```
POST /api/resumes/:id/pdf
  → compile IR to LaTeX (fast, in-memory)
  → hash = SHA-256(latexContent)
  → if /tmp/forge-pdf-cache/{hash}.pdf exists:
      return cached file (instant)
  → else:
      write .tex, run tectonic, read .pdf
      copy .pdf to /tmp/forge-pdf-cache/{hash}.pdf
      return PDF
  → cleanup: .tex, .log, .aux (but NOT the cached .pdf)
```

**Invalidation:** Cache is content-addressed, so it self-invalidates — different LaTeX content = different hash = cache miss. No explicit invalidation needed. Old cache files can be cleaned up on a timer or at server start.

**Regenerate button:** Passes a `?bust=1` query parameter that skips the cache check, forces recompile, and overwrites the cached file.

### Response Headers

Add `X-Forge-Pdf-Cache: hit|miss` header so the client can show cache status (optional, useful for debugging).

## Component Changes

### Remove

- `PdfView.svelte` — replaced by unified preview
- `SourceView.svelte` — replaced by unified editor
- "Generate PDF" button — auto-render replaces it
- 3-second debounce `$effect` on `ir` in PdfView — no longer needed; preview fetches from cache on tab open

### Create

- `ResumePreview.svelte` — unified preview component
  - If format is `default` or `latex`: render PDF iframe from `POST /api/resumes/:id/pdf`
  - If format is `markdown`: render GFM client-side from IR markdown content
  - Toolbar: regenerate icon button, download icon button
  - Loading state: spinner while PDF loads (no "Generate" button)

- `ResumeEditor.svelte` — unified editor component
  - If format is `default`: render `<DragNDropView>`
  - If format is `latex`: render CodeMirror with LaTeX syntax, save/discard buttons, linter integration
  - If format is `markdown`: render CodeMirror with Markdown syntax, save/discard buttons, linter integration
  - Dirty state tracking for LaTeX/Markdown modes

- `FormatToggle.svelte` — format selector (default/latex/markdown)
  - Renders as segmented control or dropdown
  - State lives in the parent page, passed down to both Editor and Preview
  - Warns if switching format with unsaved changes

### Modify

- `+page.svelte` (resumes route)
  - Replace 3-tab system with 2-tab (Editor/Preview)
  - Add format toggle above tabs (or in toolbar)
  - Pass format state to both child components
  - Use `<TabBar>` shared component (fixes existing rule violation)

- `resume-service.ts` (core)
  - Add cache directory init (`/tmp/forge-pdf-cache/`)
  - Add SHA-256 hash computation before tectonic invocation
  - Add cache read/write logic around existing tectonic flow
  - Add `bust` query parameter support

- `resumes.ts` route (core)
  - Pass `bust` parameter from request to service

- `sb2nov.ts` (LaTeX template)
  - Add `\hypersetup{pdfpagemode=UseNone}` to suppress document outline panel

## LaTeX/Markdown Linting

### LaTeX Linter

Before saving a LaTeX override:
- Compile with tectonic in a dry-run or validation mode
- Check for compilation errors
- If errors: show inline in CodeMirror, block save
- If clean: save to `latex_override`, invalidate cache

### Markdown Linter

Before saving a Markdown override:
- Parse with the GFM renderer
- Check for structural issues (unclosed tags, broken links)
- Lighter validation than LaTeX — Markdown is more forgiving

## Document Outline Fix

Add to `sb2nov.ts` preamble:

```latex
\hypersetup{
  hidelinks,
  pdfpagemode=UseNone
}
```

This replaces the current `\usepackage[hidelinks]{hyperref}` and adds `pdfpagemode=UseNone` to suppress the PDF outline/bookmarks panel in viewers.

## Markdown Preview Rendering

Use a client-side GFM renderer (e.g., `marked` or `markdown-it` with GFM plugin). The IR provides the markdown content via:
- `markdown_override` if set
- Otherwise, generated from IR via the existing `generateMarkdown()` function

Render in a styled container matching the resume's visual style (or a clean reading view). No server round-trip needed.

## Migration Path

1. Fix document outline (`\hypersetup`) — one-line change, immediate value
2. Add server-side PDF cache — backend only, no UI changes needed
3. Build `ResumePreview.svelte` with auto-render from cache
4. Build `FormatToggle.svelte`
5. Build `ResumeEditor.svelte` with format-aware editor switching
6. Wire into `+page.svelte` replacing the 3-tab system
7. Add LaTeX/Markdown save + lint flow
8. Remove old components (`PdfView`, `SourceView`)

## Decisions Made

- **Preview always renders from IR** — no live preview of unsaved edits. Simplifies caching and prevents partial/broken renders.
- **Server-side content-addressed cache** — self-invalidating, no explicit invalidation logic needed.
- **Format is global** — persists across Editor/Preview switches. Prevents confusion about what you're looking at.
- **DnD edits are live, LaTeX/Markdown require save** — DnD is the primary workflow and should feel immediate. LaTeX/Markdown are power-user features with explicit save semantics.
- **GFM for Markdown** — GitHub-Flavored Markdown is the standard Adam uses. Client-side rendering, no server call.
