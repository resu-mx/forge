# Phase 45: Editor Restructuring

**Status:** Planning
**Date:** 2026-04-03
**Spec:** [2026-04-03-editor-restructuring.md](../refs/specs/2026-04-03-editor-restructuring.md)
**Depends on:** None (existing tab infrastructure is in place)
**Blocks:** None currently identified
**Parallelizable with:** Phase 44 (IR Data Quality), Phase 46 (LaTeX/XeTeX Docs), Phase 47 (Clearance Structured Data) -- no code overlap

## Goal

Restructure the resume detail page tabs from four confusing tabs (`DragNDrop | Markdown | LaTeX | PDF`) into three intuitive tabs (`Editor | Preview | Source`). The Source tab combines the LaTeX and Markdown views with a format toggle (LaTeX/Markdown) and a mode toggle (View/Edit), using a single CodeMirror instance that swaps language mode via `Compartment` reconfiguration. This eliminates the conflation of editing views with output format views and creates a clearer navigation flow.

## Non-Goals

- IR data fixes (see Spec F1 / Phase 44)
- Template changes (sb2nov.ts rendering logic)
- New output formats beyond LaTeX and Markdown
- Server-side LaTeX compilation (the Source tab works with client-side generated content or stored overrides)
- URL-based tab state persistence
- Component-level automated testing infrastructure
- Mobile-responsive layout for the source sub-bar

## Context

The current `/resumes` page has four top-level tabs:

| Tab | What it does |
|-----|-------------|
| `DragNDrop` | Primary editor -- drag-and-drop section/entry editor |
| `Markdown` | Shows Markdown source with Preview/Edit sub-tabs |
| `LaTeX` | Shows LaTeX source with Preview/Edit sub-tabs |
| `PDF` | PDF preview of compiled resume |

This layout conflates editing views (DragNDrop) with output format views (Markdown, LaTeX) and a preview (PDF). Users must understand that DragNDrop is the primary editor, Markdown/LaTeX are source views with optional editing, and PDF is a preview. The restructuring reframes these into three clear categories:

| New Tab | Purpose | Maps to |
|---------|---------|---------|
| **Editor** (default) | Drag-and-drop section/entry editor | `DragNDrop` |
| **Preview** | PDF preview | `PDF` |
| **Source** | View/edit LaTeX or Markdown | `Markdown` + `LaTeX` combined |

The Source tab has two sub-controls: format toggle (LaTeX/Markdown) and mode toggle (View/Edit). The CodeMirror instance stays alive across format switches via `Compartment` reconfiguration.

## Scope

| Spec section | Covered here? |
|-------------|---------------|
| 1. Tab type and navigation | Yes |
| 2. Source tab sub-controls | Yes |
| 3.1 New SourceView.svelte | Yes |
| 3.2 Retain existing components | Yes |
| 4. Tab content rendering | Yes |
| 5. Sub-toggle UI design | Yes |
| 6. Generated content for View mode | Yes |
| 7. State persistence | Yes |

## Files to Create

| File | Description |
|------|-------------|
| `packages/webui/src/lib/components/resume/SourceView.svelte` | Unified source view with format/mode toggles and CodeMirror |

## Files to Modify

| File | Change |
|------|--------|
| `packages/webui/src/routes/resumes/+page.svelte` | Replace `ViewTab` type and `VIEW_TABS` array; update tab rendering conditional; replace LatexView/MarkdownView imports with SourceView; pass both override props to SourceView |
| `packages/webui/src/lib/components/resume/index.ts` | Export `SourceView`; keep LatexView/MarkdownView exports (not deleted, just no longer imported on resumes page) |

## Fallback Strategies

- **CodeMirror Compartment reconfiguration fails:** If language swap via `Compartment.reconfigure()` does not work as expected, fall back to destroying and recreating the `EditorView` on format switch. This loses cursor position but is functionally correct.
- **`@codemirror/lang-markdown` not installed:** The existing `MarkdownView.svelte` already imports Markdown language support. If the package is missing, the Source tab can fall back to plain text mode for Markdown (no syntax highlighting).
- **Existing LatexView/MarkdownView references:** These components remain in the codebase and in the barrel export. Any external consumers (if any) continue to work. Only the resumes page stops importing them directly.
- **Override banner parameterization:** The existing `OverrideBanner.svelte` component already accepts `overrideUpdatedAt` and `resumeUpdatedAt` props. No changes to the banner component are needed -- `SourceView` passes the correct timestamps based on the active format.
- **CodeMirror SSR:** Svelte 5 components using `onMount` already guard against SSR. The CodeMirror instance is only created in the browser.

---

## Tasks

### T45.1: Create `SourceView.svelte` [CRITICAL]

**File:** `packages/webui/src/lib/components/resume/SourceView.svelte`

Create a unified source view component that handles both LaTeX and Markdown formats with View/Edit mode toggle. Uses a single CodeMirror instance with `Compartment` reconfiguration for language switching.

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { EditorView, keymap } from '@codemirror/view'
  import { EditorState, Compartment } from '@codemirror/state'
  import { basicSetup } from 'codemirror'
  import { StreamLanguage } from '@codemirror/language'
  import { stex } from '@codemirror/legacy-modes/mode/stex'
  import { markdown } from '@codemirror/lang-markdown'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import OverrideBanner from './OverrideBanner.svelte'
  import type { ResumeDocument } from '@forge/sdk'

  type SourceFormat = 'latex' | 'markdown'
  type SourceMode = 'view' | 'edit'

  let {
    ir,
    resumeId,
    latexOverride,
    latexOverrideUpdatedAt,
    markdownOverride,
    markdownOverrideUpdatedAt,
    resumeUpdatedAt,
    onOverrideChange,
  }: {
    ir: ResumeDocument
    resumeId: string
    latexOverride: string | null
    latexOverrideUpdatedAt: string | null
    markdownOverride: string | null
    markdownOverrideUpdatedAt: string | null
    resumeUpdatedAt: string
    onOverrideChange: () => Promise<void>
  } = $props()

  let sourceFormat = $state<SourceFormat>('latex')
  let sourceMode = $state<SourceMode>('view')
  let saving = $state(false)

  let editorContainer: HTMLDivElement | undefined = $state()
  let editorView: EditorView | undefined = $state()

  // Compartments for dynamic reconfiguration
  const languageCompartment = new Compartment()
  const editableCompartment = new Compartment()
  const readOnlyCompartment = new Compartment()

  // Derived: current override and timestamps based on format
  let currentOverride = $derived(
    sourceFormat === 'latex' ? latexOverride : markdownOverride
  )
  let currentOverrideUpdatedAt = $derived(
    sourceFormat === 'latex' ? latexOverrideUpdatedAt : markdownOverrideUpdatedAt
  )
  let isOverride = $derived(currentOverride !== null)
  let isStale = $derived(
    isOverride && currentOverrideUpdatedAt !== null && resumeUpdatedAt > currentOverrideUpdatedAt
  )

  // Content for display
  let displayContent = $derived(
    currentOverride ?? (sourceFormat === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdown(ir))
  )

  let isEditable = $derived(sourceMode === 'edit')

  function getLanguageExtension(format: SourceFormat) {
    return format === 'latex'
      ? StreamLanguage.define(stex)
      : markdown()
  }

  function createEditor() {
    if (!editorContainer) return

    const startState = EditorState.create({
      doc: displayContent,
      extensions: [
        basicSetup,
        languageCompartment.of(getLanguageExtension(sourceFormat)),
        editableCompartment.of(EditorView.editable.of(isEditable)),
        readOnlyCompartment.of(EditorState.readOnly.of(!isEditable)),
        EditorView.lineWrapping,
        EditorView.theme({
          '&': { fontSize: '0.82rem' },
          '.cm-content': { fontFamily: '"Fira Code", "JetBrains Mono", monospace' },
          '.cm-gutters': { background: '#f9fafb', borderRight: '1px solid #e5e7eb' },
        }),
      ],
    })

    editorView = new EditorView({
      state: startState,
      parent: editorContainer,
    })
  }

  function destroyEditor() {
    editorView?.destroy()
    editorView = undefined
  }

  // Swap language and content when format changes
  $effect(() => {
    if (!editorView) return
    const lang = getLanguageExtension(sourceFormat)
    editorView.dispatch({
      effects: languageCompartment.reconfigure(lang),
    })
    // Replace document content
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: displayContent,
      },
    })
  })

  // Update editability when mode changes
  $effect(() => {
    if (!editorView) return
    editorView.dispatch({
      effects: [
        editableCompartment.reconfigure(EditorView.editable.of(isEditable)),
        readOnlyCompartment.reconfigure(EditorState.readOnly.of(!isEditable)),
      ],
    })
  })

  onMount(() => {
    createEditor()
  })

  onDestroy(() => {
    destroyEditor()
  })

  // ---- Actions ----

  async function handleSave() {
    if (!editorView) return
    saving = true
    try {
      const content = editorView.state.doc.toString()
      const field = sourceFormat === 'latex' ? 'latex_override' : 'markdown_override'
      const result = await forge.resumes.update(resumeId, { [field]: content })
      if (result.ok) {
        addToast('Saved', 'success')
        await onOverrideChange()
      } else {
        addToast(friendlyError(result.error), 'error')
      }
    } catch (err) {
      addToast('Failed to save', 'error')
    } finally {
      saving = false
    }
  }

  async function handleRegenerate() {
    if (!editorView) return
    saving = true
    try {
      const generated = sourceFormat === 'latex'
        ? generateLatexPlaceholder(ir)
        : generateMarkdown(ir)
      editorView.dispatch({
        changes: {
          from: 0,
          to: editorView.state.doc.length,
          insert: generated,
        },
      })
      addToast('Regenerated from IR', 'success')
    } finally {
      saving = false
    }
  }

  async function handleReset() {
    saving = true
    try {
      const field = sourceFormat === 'latex' ? 'latex_override' : 'markdown_override'
      const result = await forge.resumes.update(resumeId, { [field]: null })
      if (result.ok) {
        addToast('Override cleared', 'success')
        await onOverrideChange()
        // Update editor content to generated
        if (editorView) {
          const generated = sourceFormat === 'latex'
            ? generateLatexPlaceholder(ir)
            : generateMarkdown(ir)
          editorView.dispatch({
            changes: {
              from: 0,
              to: editorView.state.doc.length,
              insert: generated,
            },
          })
        }
      } else {
        addToast(friendlyError(result.error), 'error')
      }
    } catch (err) {
      addToast('Failed to reset', 'error')
    } finally {
      saving = false
    }
  }

  // ---- Content generators (reused from existing LatexView/MarkdownView) ----

  function escLatex(text: string): string {
    return text
      .replace(/\\/g, '\\textbackslash{}')
      .replace(/[&%$#_{}]/g, (c) => `\\${c}`)
      .replace(/~/g, '\\textasciitilde{}')
      .replace(/\^/g, '\\textasciicircum{}')
  }

  function generateLatexPlaceholder(doc: ResumeDocument): string {
    const lines: string[] = []
    lines.push('% Auto-generated LaTeX preview from resume IR')
    lines.push('% Enable editing and save to create an override')
    lines.push('\\documentclass[letterpaper,11pt]{article}')
    lines.push('\\begin{document}')
    lines.push('')
    lines.push(`\\section*{${escLatex(doc.header.name)}}`)
    if (doc.header.tagline) lines.push(`\\textit{${escLatex(doc.header.tagline)}}`)
    const contact: string[] = []
    if (doc.header.location) contact.push(escLatex(doc.header.location))
    if (doc.header.email) contact.push(escLatex(doc.header.email))
    if (doc.header.phone) contact.push(escLatex(doc.header.phone))
    if (contact.length > 0) lines.push(contact.join(' $\\cdot$ '))
    lines.push('')

    for (const section of doc.sections) {
      lines.push(`\\section{${escLatex(section.title)}}`)
      for (const item of section.items) {
        if (item.kind === 'experience_group') {
          lines.push('\\resumeSubHeadingListStart')
          for (const sub of item.subheadings) {
            lines.push(`\\resumeSubheading{${escLatex(item.organization)}}{}{${escLatex(sub.title)}}{${escLatex(sub.date_range)}}`)
            lines.push('\\resumeItemListStart')
            for (const b of sub.bullets) {
              lines.push(`\\resumeItem{${escLatex(b.content)}}`)
            }
            lines.push('\\resumeItemListEnd')
          }
          lines.push('\\resumeSubHeadingListEnd')
        } else if (item.kind === 'summary') {
          lines.push(escLatex(item.content))
        } else if (item.kind === 'education') {
          lines.push(`\\resumeSubheading{${escLatex(item.institution)}}{${escLatex(item.location ?? '')}}{${escLatex(item.degree)}}{${escLatex(item.date)}}`)
        } else if (item.kind === 'skill_group') {
          for (const cat of item.categories) {
            lines.push(`\\textbf{${escLatex(cat.label)}:} ${cat.skills.map(escLatex).join(', ')}`)
          }
        } else if (item.kind === 'project') {
          lines.push(`\\resumeProjectHeading{${escLatex(item.name)}}{${escLatex(item.date ?? '')}}`)
          if (item.bullets.length > 0) {
            lines.push('\\resumeItemListStart')
            for (const b of item.bullets) {
              lines.push(`\\resumeItem{${escLatex(b.content)}}`)
            }
            lines.push('\\resumeItemListEnd')
          }
        } else if (item.kind === 'clearance') {
          lines.push(escLatex(item.content))
        } else if (item.kind === 'certification_group') {
          for (const cat of item.categories) {
            lines.push(`\\textbf{${escLatex(cat.label)}:} ${cat.certs.map(c => escLatex(c.name)).join(', ')}`)
          }
        } else if (item.kind === 'presentation') {
          lines.push(`\\resumeProjectHeading{${escLatex(item.title)}}{${escLatex(item.date ?? '')}}`)
          if (item.bullets.length > 0) {
            lines.push('\\resumeItemListStart')
            for (const b of item.bullets) {
              lines.push(`\\resumeItem{${escLatex(b.content)}}`)
            }
            lines.push('\\resumeItemListEnd')
          }
        }
      }
    }

    lines.push('')
    lines.push('\\end{document}')
    return lines.join('\n')
  }

  function generateMarkdown(doc: ResumeDocument): string {
    const lines: string[] = []
    lines.push(`# ${doc.header.name}`)
    if (doc.header.tagline) lines.push(`*${doc.header.tagline}*`)
    const contact: string[] = []
    if (doc.header.location) contact.push(doc.header.location)
    if (doc.header.email) contact.push(doc.header.email)
    if (doc.header.phone) contact.push(doc.header.phone)
    if (doc.header.linkedin) contact.push(`[LinkedIn](${doc.header.linkedin})`)
    if (doc.header.github) contact.push(`[GitHub](${doc.header.github})`)
    if (doc.header.website) contact.push(`[Website](${doc.header.website})`)
    if (contact.length > 0) lines.push(contact.join(' | '))
    lines.push('')

    for (const section of doc.sections) {
      lines.push(`## ${section.title}`)
      lines.push('')
      for (const item of section.items) {
        if (item.kind === 'experience_group') {
          lines.push(`### ${item.organization}`)
          for (const sub of item.subheadings) {
            lines.push(`**${sub.title}** | ${sub.date_range}`)
            lines.push('')
            for (const b of sub.bullets) {
              lines.push(`- ${b.content}`)
            }
            lines.push('')
          }
        } else if (item.kind === 'summary') {
          lines.push(item.content)
          lines.push('')
        } else if (item.kind === 'education') {
          lines.push(`**${item.institution}**${item.location ? ` | ${item.location}` : ''}`)
          lines.push(`${item.degree} | ${item.date}`)
          lines.push('')
        } else if (item.kind === 'skill_group') {
          for (const cat of item.categories) {
            lines.push(`**${cat.label}:** ${cat.skills.join(', ')}`)
          }
          lines.push('')
        } else if (item.kind === 'project') {
          lines.push(`**${item.name}**${item.date ? ` | ${item.date}` : ''}`)
          for (const b of item.bullets) {
            lines.push(`- ${b.content}`)
          }
          lines.push('')
        } else if (item.kind === 'clearance') {
          lines.push(`- ${item.content}`)
        } else if (item.kind === 'certification_group') {
          for (const cat of item.categories) {
            lines.push(`**${cat.label}:** ${cat.certs.map(c => c.name).join(', ')}`)
          }
          lines.push('')
        } else if (item.kind === 'presentation') {
          lines.push(`**${item.title}**${item.date ? ` | ${item.date}` : ''}`)
          for (const b of item.bullets) {
            lines.push(`- ${b.content}`)
          }
          lines.push('')
        }
      }
    }

    return lines.join('\n')
  }
</script>

<div class="source-view">
  <!-- Override banner -->
  {#if isOverride}
    <OverrideBanner
      {isStale}
      overrideUpdatedAt={currentOverrideUpdatedAt}
      {resumeUpdatedAt}
    />
  {/if}

  <!-- Sub-bar: format toggle + mode toggle + actions -->
  <div class="source-toolbar">
    <div class="toggle-group">
      <span class="toggle-label">Format:</span>
      <button
        class="toggle-btn"
        class:active={sourceFormat === 'latex'}
        onclick={() => sourceFormat = 'latex'}
      >LaTeX</button>
      <button
        class="toggle-btn"
        class:active={sourceFormat === 'markdown'}
        onclick={() => sourceFormat = 'markdown'}
      >Markdown</button>
    </div>

    <div class="toggle-group">
      <span class="toggle-label">Mode:</span>
      <button
        class="toggle-btn"
        class:active={sourceMode === 'view'}
        onclick={() => sourceMode = 'view'}
      >View</button>
      <button
        class="toggle-btn"
        class:active={sourceMode === 'edit'}
        onclick={() => sourceMode = 'edit'}
      >Edit</button>
    </div>

    {#if sourceMode === 'edit'}
      <div class="action-group">
        <button class="action-btn save" onclick={handleSave} disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
        <button class="action-btn" onclick={handleRegenerate} disabled={saving}>
          Regenerate
        </button>
        {#if isOverride}
          <button class="action-btn danger" onclick={handleReset} disabled={saving}>
            Reset
          </button>
        {/if}
      </div>
    {/if}
  </div>

  <!-- CodeMirror editor -->
  <div class="editor-wrapper" bind:this={editorContainer}></div>
</div>

<style>
  .source-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .source-toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
    flex-wrap: wrap;
  }

  .toggle-group {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .toggle-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    margin-right: 0.15rem;
  }

  .toggle-btn {
    padding: 0.3rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    background: #fff;
    color: #374151;
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.12s;
  }

  .toggle-btn:hover {
    background: #f3f4f6;
  }

  .toggle-btn.active {
    background: #6c63ff;
    color: #fff;
    border-color: #6c63ff;
  }

  .action-group {
    display: flex;
    gap: 0.35rem;
    margin-left: auto;
  }

  .action-btn {
    padding: 0.3rem 0.6rem;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    background: #fff;
    color: #374151;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.12s;
  }

  .action-btn:hover {
    background: #f3f4f6;
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn.save {
    background: #6c63ff;
    color: #fff;
    border-color: #6c63ff;
  }

  .action-btn.save:hover {
    background: #5a52e0;
  }

  .action-btn.danger {
    color: #dc2626;
    border-color: #fca5a5;
  }

  .action-btn.danger:hover {
    background: #fef2f2;
  }

  .editor-wrapper {
    flex: 1;
    overflow: auto;
  }

  .editor-wrapper :global(.cm-editor) {
    height: 100%;
  }
</style>
```

**Acceptance criteria:**
- Component renders with format toggle (LaTeX/Markdown) and mode toggle (View/Edit).
- LaTeX format shows LaTeX syntax highlighting via `stex`.
- Markdown format shows Markdown syntax highlighting via `@codemirror/lang-markdown`.
- View mode: CodeMirror is read-only (cannot type, cursor shows but no editing).
- Edit mode: CodeMirror is editable with Save/Regenerate/Reset buttons.
- Format switch preserves mode (if in Edit mode on LaTeX, switching to Markdown stays in Edit mode).
- Save persists the override via the API (`latex_override` or `markdown_override`).
- Reset clears the override and reverts to generated content.
- Regenerate replaces editor content with freshly generated content from IR.
- Override banner shows when an override exists and is stale.
- Single CodeMirror instance across format switches (Compartment reconfiguration, not recreation).

**Failure criteria:**
- Two CodeMirror instances created (memory leak).
- Language highlighting does not switch on format change.
- View mode allows editing.
- Save writes to wrong override field.

---

### T45.2: Update Tab Types and Rendering in `+page.svelte` [CRITICAL]

**File:** `packages/webui/src/routes/resumes/+page.svelte`

Replace the `ViewTab` type, `VIEW_TABS` array, and tab rendering conditional. Remove LatexView/MarkdownView imports, add SourceView import.

**Current imports (lines 8-11):**
```typescript
  import DragNDropView from '$lib/components/resume/DragNDropView.svelte'
  import MarkdownView from '$lib/components/resume/MarkdownView.svelte'
  import LatexView from '$lib/components/resume/LatexView.svelte'
  import PdfView from '$lib/components/resume/PdfView.svelte'
```

**Replace with:**
```typescript
  import DragNDropView from '$lib/components/resume/DragNDropView.svelte'
  import PdfView from '$lib/components/resume/PdfView.svelte'
  import SourceView from '$lib/components/resume/SourceView.svelte'
```

**Current tab types (lines 16-23):**
```typescript
  type ViewTab = 'dnd' | 'markdown' | 'latex' | 'pdf'

  const VIEW_TABS: { value: ViewTab; label: string }[] = [
    { value: 'dnd', label: 'DragNDrop' },
    { value: 'markdown', label: 'Markdown' },
    { value: 'latex', label: 'LaTeX' },
    { value: 'pdf', label: 'PDF' },
  ]
```

**Replace with:**
```typescript
  type ViewTab = 'editor' | 'preview' | 'source'

  const VIEW_TABS: { value: ViewTab; label: string }[] = [
    { value: 'editor', label: 'Editor' },
    { value: 'preview', label: 'Preview' },
    { value: 'source', label: 'Source' },
  ]
```

**Current tab rendering conditional** (search for `activeViewTab` in the template):

The existing conditional renders different components based on the active tab. Find the block that switches on `activeViewTab` and replace:

```svelte
<!-- Before (4-way switch) -->
{#if activeViewTab === 'dnd'}
  <DragNDropView ... />
{:else if activeViewTab === 'markdown'}
  <MarkdownView ... />
{:else if activeViewTab === 'latex'}
  <LatexView ... />
{:else if activeViewTab === 'pdf'}
  <PdfView ... />
{/if}
```

```svelte
<!-- After (3-way switch) -->
{#if activeViewTab === 'editor'}
  <DragNDropView ... />
{:else if activeViewTab === 'preview'}
  <PdfView ... />
{:else if activeViewTab === 'source'}
  <SourceView
    {ir}
    {resumeId}
    latexOverride={resumeDetail.latex_override}
    latexOverrideUpdatedAt={resumeDetail.latex_override_updated_at}
    markdownOverride={resumeDetail.markdown_override}
    markdownOverrideUpdatedAt={resumeDetail.markdown_override_updated_at}
    resumeUpdatedAt={resumeDetail.updated_at}
    onOverrideChange={loadResumeDetail}
  />
{/if}
```

**Important:** The `ir` and `resumeId` variables already exist in the page component (used by the current LatexView and PdfView). The `resumeDetail` variable provides the override fields. The `loadResumeDetail` function already exists and refreshes the resume detail state.

Also update any references to `activeViewTab === 'dnd'` elsewhere in the file (e.g., for conditional rendering of edit controls) to use `activeViewTab === 'editor'`.

**Acceptance criteria:**
- Three tabs render: Editor, Preview, Source.
- Default tab is Editor (previously was `dnd`).
- Editor tab renders DragNDropView (unchanged behavior).
- Preview tab renders PdfView (unchanged behavior).
- Source tab renders SourceView with all required props.
- No references to `'dnd'`, `'markdown'`, or `'latex'` remain in the ViewTab type.
- MarkdownView and LatexView imports are removed from this file.

**Failure criteria:**
- Four tabs still render.
- Default tab is wrong.
- SourceView crashes due to missing props.
- References to old tab values cause TypeScript errors.

---

### T45.3: Update Barrel Export [MINOR]

**File:** `packages/webui/src/lib/components/resume/index.ts`

Add `SourceView` to the barrel export. Keep existing exports for backward compatibility.

**Current content:**
```typescript
export { default as DragNDropView } from './DragNDropView.svelte'
export { default as MarkdownView } from './MarkdownView.svelte'
export { default as LatexView } from './LatexView.svelte'
export { default as PdfView } from './PdfView.svelte'
export { default as HeaderEditor } from './HeaderEditor.svelte'
export { default as OverrideBanner } from './OverrideBanner.svelte'
```

**Replace with:**
```typescript
export { default as DragNDropView } from './DragNDropView.svelte'
export { default as MarkdownView } from './MarkdownView.svelte'
export { default as LatexView } from './LatexView.svelte'
export { default as PdfView } from './PdfView.svelte'
export { default as SourceView } from './SourceView.svelte'
export { default as HeaderEditor } from './HeaderEditor.svelte'
export { default as OverrideBanner } from './OverrideBanner.svelte'
```

**Acceptance criteria:**
- `SourceView` is importable from `$lib/components/resume`.
- Existing exports unchanged.

**Failure criteria:**
- Import resolution error for SourceView.

---

## Testing Support

### Test Fixtures

No test fixtures needed. This phase is UI-only with no database changes.

### Unit Tests

No unit tests. The codebase does not have a component testing framework for Svelte 5 components (acknowledged as a non-goal in the spec).

### Manual Tests

| Test | What to verify |
|------|---------------|
| Tab navigation | Click Editor/Preview/Source tabs, verify correct component renders |
| Default tab | Load resume detail page, verify Editor tab is active |
| Source format toggle | In Source tab, toggle between LaTeX and Markdown, verify content switches |
| Source mode toggle | Toggle between View and Edit, verify CodeMirror editability changes |
| View mode read-only | In View mode, verify text cannot be edited (cursor is read-only) |
| Edit mode save | In Edit mode, modify content, click Save, verify override persists |
| Override banner | Verify stale override banner appears when resume updated_at > override updated_at |
| Reset override | Click Reset, verify override is cleared and content reverts to generated |
| Regenerate | Click Regenerate, verify content is replaced with fresh IR-generated content |
| Format switch preserves mode | LaTeX/Edit -> switch to Markdown -> still in Edit mode |
| Tab switch resets source state | Switch to Editor tab and back to Source, verify format/mode reset to defaults (latex/view) |
| LaTeX syntax highlighting | In LaTeX format, verify backslash commands are highlighted |
| Markdown syntax highlighting | In Markdown format, verify headers/bold/lists are highlighted |
| Save to correct field | Save in LaTeX mode -> `latex_override` updated; Save in Markdown mode -> `markdown_override` updated |

### Smoke Tests

| Test | What to verify |
|------|---------------|
| Page loads without error | No console errors, all three tabs clickable |
| DnD editing still works | Edit entries in Editor tab, verify persistence |
| PDF preview still works | Preview tab shows PDF |
| Override roundtrip | Save LaTeX override, switch to Preview, switch back to Source, verify override is still there |

### Visual Tests

| Test | What to verify |
|------|---------------|
| Source toolbar layout | Format/Mode toggles aligned horizontally, action buttons on right |
| Active toggle button | Active toggle has purple background (#6c63ff), inactive is white |
| Save button styling | Green/purple accent, disabled state has reduced opacity |
| Danger button styling | Reset button has red text and red border accent |

---

## Documentation Requirements

- No new documentation files required.
- Inline JSDoc on `SourceView.svelte` props explains each override field.
- The spec file serves as the design document for the tab restructuring rationale.

---

## Parallelization Notes

**Within this phase:**
- T45.1 (SourceView component) must be done before T45.2 (page update) -- the page imports SourceView.
- T45.3 (barrel export) can be done in parallel with T45.2 or after.

**Recommended execution order:**
1. T45.1 (create SourceView component)
2. T45.2 + T45.3 (update page + barrel export)

**Cross-phase:**
- Phase 44 (IR Data Quality) changes the data flowing into `ResumeDocument` but does not change the IR type structure. SourceView consumes the IR for content generation, so Phase 44 improvements will automatically appear in the Source tab.
- Phase 46 (LaTeX/XeTeX Docs) is documentation-only.
- Phase 47 (Clearance Structured Data) has no UI overlap with this phase.
- SourceView.svelte should use design tokens from Phase 42 (`var(--color-surface)`, `var(--color-border)`, etc.) instead of hardcoded hex values. If Phase 42 has not landed yet, use hardcoded values with a TODO comment for token migration.
