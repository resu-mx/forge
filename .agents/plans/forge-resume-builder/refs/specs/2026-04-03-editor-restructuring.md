# Resume Editor Restructuring

**Date:** 2026-04-03
**Spec:** F2 (Editor Restructuring)
**Phase:** TBD
**Builds on:** Current resume page tabs (`DragNDrop | Markdown | LaTeX | PDF`)
**Related:** [IR Data Quality](2026-04-03-ir-data-quality.md)

## Overview

The resume detail page currently has four top-level tabs: `DragNDrop`, `Markdown`, `LaTeX`, and `PDF`. This layout conflates editing views with output format views and creates an unintuitive navigation flow. Users must understand that DragNDrop is the primary editor, Markdown/LaTeX are source views with optional editing, and PDF is a preview.

This spec restructures the tabs into three categories that better map to user intent:

| Tab | Purpose | Current equivalent |
|-----|---------|-------------------|
| **Editor** (default) | Drag-and-drop section/entry editor | `DragNDrop` tab |
| **Preview** | PDF preview of the compiled resume | `PDF` tab |
| **Source** | View/edit the raw LaTeX or Markdown output | `Markdown` + `LaTeX` tabs combined |

The **Source** tab introduces two toggles:
- **Format toggle:** LaTeX / Markdown (selects which output format to display)
- **Mode toggle:** View / Edit (switches between read-only syntax-highlighted view and editable CodeMirror)

## Scope

Restructure the resume page tab bar and consolidate existing components. No changes to the IR compiler, template rendering, or data layer.

## Technical Approach

### 1. Tab type and navigation

Replace the current `ViewTab` type and `VIEW_TABS` array:

```typescript
// Before
type ViewTab = 'dnd' | 'markdown' | 'latex' | 'pdf'
const VIEW_TABS = [
  { value: 'dnd', label: 'DragNDrop' },
  { value: 'markdown', label: 'Markdown' },
  { value: 'latex', label: 'LaTeX' },
  { value: 'pdf', label: 'PDF' },
]

// After
type ViewTab = 'editor' | 'preview' | 'source'
const VIEW_TABS: { value: ViewTab; label: string }[] = [
  { value: 'editor', label: 'Editor' },
  { value: 'preview', label: 'Preview' },
  { value: 'source', label: 'Source' },
]
```

Default tab remains the editor (was `dnd`, now `editor`).

### 2. Source tab sub-controls

When the `source` tab is active, render two toggle groups below the main tab bar:

```typescript
type SourceFormat = 'latex' | 'markdown'
type SourceMode = 'view' | 'edit'

let sourceFormat = $state<SourceFormat>('latex')
let sourceMode = $state<SourceMode>('view')
```

**Format toggle** (`LaTeX | Markdown`): Switches between the LaTeX and Markdown source representations. Each format has its own override state (`latex_override` / `markdown_override` on the resume). Changing format preserves the mode.

**Mode toggle** (`View | Edit`):
- **View mode:** Read-only, syntax-highlighted display. Uses the same CodeMirror instance but with `EditorView.editable.of(false)` and `EditorState.readOnly.of(true)`. This replaces the current "Preview" sub-tab inside LatexView/MarkdownView (which renders parsed HTML). The rationale: showing the actual source with syntax highlighting is more useful for review than a lossy HTML approximation.
- **Edit mode:** Full CodeMirror editing with save/regenerate/reset actions. Same behavior as the current Edit sub-tab in LatexView/MarkdownView.

### 3. Component refactoring

#### 3.1 New `SourceView.svelte`

Create a unified source view component that handles both formats:

```typescript
// Props
let {
  ir,
  resumeId,
  latexOverride,
  latexOverrideUpdatedAt,
  markdownOverride,
  markdownOverrideUpdatedAt,
  resumeUpdatedAt,
  onOverrideChange,
}: { ... } = $props()
```

Internally, `SourceView` manages:
- Format toggle state (`latex` / `markdown`)
- Mode toggle state (`view` / `edit`)
- A single CodeMirror instance that swaps its language extension and content when the format changes
- Override banner (reuses existing `OverrideBanner.svelte`). The OverrideBanner is parameterized by the current format: in LaTeX mode, compare against `latex_override_updated_at`; in Markdown mode, compare against `markdown_override_updated_at`.
- Save/regenerate/reset actions scoped to the active format

The CodeMirror language mode switches between:
- `StreamLanguage.define(stex)` for LaTeX
- `markdown()` from `@codemirror/lang-markdown` for Markdown

CodeMirror 6 language extension swap must use `Compartment` reconfiguration (`languageCompartment.reconfigure(newLanguage)`), not editor recreation. This preserves focus, cursor position, and scroll state across format switches.

When switching formats, the editor content is replaced with the appropriate override or generated content.

#### 3.2 Retain existing components

- `DragNDropView.svelte` -- unchanged, renamed tab label from "DragNDrop" to "Editor"
- `PdfView.svelte` -- unchanged, renamed tab label from "PDF" to "Preview"
- `LatexView.svelte` -- kept for now but no longer directly mounted as a tab. Its CodeMirror setup, linting, and save logic are extracted into `SourceView.svelte` or shared utilities.
- `MarkdownView.svelte` -- same treatment as LatexView.

In a later cleanup pass, `LatexView.svelte` and `MarkdownView.svelte` can be removed once `SourceView.svelte` fully subsumes their functionality. For this spec, they remain in the codebase but are no longer imported into the resumes page.

### 4. Tab content rendering

Update the conditional rendering block in `+page.svelte`:

```svelte
{#if activeViewTab === 'editor'}
  <DragNDropView ... />
{:else if activeViewTab === 'preview'}
  <PdfView ... />
{:else if activeViewTab === 'source'}
  <SourceView ... />
{/if}
```

### 5. Sub-toggle UI design

The source tab's sub-controls render as a secondary toolbar below the main tab bar:

```
[ Editor | Preview | Source ]                    <-- main tabs
  Format: [LaTeX] [Markdown]   Mode: [View] [Edit]   [Save]   <-- source sub-bar (only when Source active)
  ┌──────────────────────────────────────────────────────────┐
  │ (CodeMirror editor or read-only view)                    │
  └──────────────────────────────────────────────────────────┘
```

The sub-bar uses the same visual style as the existing tab bar (GitHub-style pill buttons). The Save button appears only in Edit mode.

### 6. Generated content for View mode

When no override exists and the user is in View mode:
- **LaTeX format:** Call the existing IR compiler API (`GET /api/resumes/:id/compile-ir` → `compileToLatex`) to get the compiled LaTeX string, or use the client-side `generateLatexPlaceholder()` function as a fallback. Note: this is a text generation call, not pdflatex/tectonic invocation — the Non-Goals exclusion of "server-side LaTeX compilation" refers to PDF compilation (pdflatex/tectonic), not LaTeX text generation.
- **Markdown format:** Use the client-side `generateMarkdown()` function from the existing MarkdownView.

When an override exists, View mode shows the override content (read-only).

### 7. State persistence

The active source format and mode are stored in component state only (not URL params or localStorage). When navigating away from the Source tab and back, the format/mode reset to defaults (`latex` / `view`). This keeps the implementation simple.

## Files to Create

| File | Purpose |
|------|---------|
| `packages/webui/src/lib/components/resume/SourceView.svelte` | Unified source view with format/mode toggles and CodeMirror |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/resumes/+page.svelte` | Replace `ViewTab` type and `VIEW_TABS` array; update tab rendering conditional; replace LatexView/MarkdownView imports with SourceView; pass both override props to SourceView |
| `packages/webui/src/lib/components/resume/index.ts` | Export `SourceView`; optionally deprecate LatexView/MarkdownView exports |

## Testing Approach

### Manual testing

1. **Tab navigation:** Click Editor/Preview/Source tabs, verify correct component renders
2. **Default tab:** Load resume detail page, verify Editor tab is active
3. **Source format toggle:** In Source tab, toggle between LaTeX and Markdown, verify content switches
4. **Source mode toggle:** Toggle between View and Edit, verify CodeMirror editability changes
5. **View mode read-only:** In View mode, verify text cannot be edited (cursor is read-only)
6. **Edit mode save:** In Edit mode, modify content, click Save, verify override persists
7. **Override banner:** Verify stale override banner appears when resume updated_at > override updated_at
8. **Reset override:** Click Reset, verify override is cleared and content reverts to generated
9. **Format switch preserves mode:** Verify all 4 format/mode combinations render correctly: LaTeX/View, LaTeX/Edit, Markdown/View, Markdown/Edit. Also test: format switch preserves mode in both directions (LaTeX Edit → Markdown preserves Edit, Markdown View → LaTeX preserves View)
10. **Tab switch preserves nothing:** Switch to Editor tab and back to Source, verify format/mode reset to defaults

### Automated tests

- Component unit tests are not practical for Svelte 5 components in this codebase (no component testing framework set up). Testing is manual.
- Verify no regressions in existing PDF generation flow.

## Acceptance Criteria

1. Resume detail page shows three tabs: Editor, Preview, Source
2. Editor tab renders the DragNDropView component (unchanged behavior)
3. Preview tab renders the PdfView component (unchanged behavior)
4. Source tab shows format toggle (LaTeX / Markdown) and mode toggle (View / Edit)
5. Source View mode displays syntax-highlighted read-only content via CodeMirror
6. Source Edit mode allows editing with Save/Regenerate/Reset actions
7. Source tab respects existing override mechanism (latex_override / markdown_override)
8. Override banner displays correctly for stale overrides in Source tab
9. No regressions in DragNDrop editing, PDF generation, or override save/reset flows
10. LatexView and MarkdownView components remain in the codebase (not deleted) but are no longer imported on the resumes page

## Non-Goals

- IR data fixes (see Spec F1)
- Template changes (sb2nov.ts rendering logic)
- New output formats beyond LaTeX and Markdown
- Server-side LaTeX compilation (the Source tab works with client-side generated content or stored overrides)
- URL-based tab state persistence
- Component-level automated testing infrastructure
- Mobile-responsive layout for the source sub-bar
