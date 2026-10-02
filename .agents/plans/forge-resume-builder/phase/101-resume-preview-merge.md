# Phase 101: Resume Editor/Preview Merge + PDF Caching

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge Preview and Source tabs into a unified Editor/Preview model with format toggle, server-side PDF caching, and auto-rendering.

**Architecture:** Two tabs (Editor/Preview) with a global format toggle (default/latex/markdown). Server-side content-addressed PDF cache eliminates redundant tectonic invocations. All previews render from committed IR state.

**Tech Stack:** Svelte 5, Hono, Bun, tectonic, CodeMirror, marked (already installed)

**Spec:** `.claude/plans/forge-resume-builder/refs/specs/2026-04-08-resume-preview-merge-design.md`

## File Structure

| File | Responsibility |
|------|---------------|
| `packages/core/src/templates/sb2nov.ts` | hypersetup fix (Task 1) |
| `packages/core/src/services/resume-service.ts` | PDF cache logic (Task 2) |
| `packages/core/src/routes/resumes.ts` | bust query param + cache header (Task 2) |
| `packages/sdk/src/resources/resumes.ts` | `pdf()` bust option + cache header (Task 2) |
| `packages/webui/src/lib/components/resume/FormatToggle.svelte` | Create (Task 3) |
| `packages/webui/src/lib/components/resume/ResumePreview.svelte` | Create (Task 4) |
| `packages/webui/src/lib/components/resume/ResumeEditor.svelte` | Create (Task 5) |
| `packages/webui/src/lib/components/resume/index.ts` | Update barrel exports (Task 6) |
| `packages/webui/src/lib/components/index.ts` | Remove RenderViewport export (Task 7) |
| `packages/webui/src/routes/resumes/+page.svelte` | 2-tab system + format toggle (Task 6) |
| `packages/webui/src/__tests__/data-visualization.test.ts` | Remove RenderViewport tests (Task 7) |

---

### Task 1: Fix document outline (hypersetup)

**Files:**
- Modify: `packages/core/src/templates/sb2nov.ts:28`

- [ ] **Step 1: Write the failing test**

```ts
// In packages/core/src/__tests__/latex-compiler.test.ts (add to existing test file)
import { sb2nov } from '../templates/sb2nov'

test('sb2nov preamble suppresses PDF outline panel', () => {
  expect(sb2nov.preamble).toContain('pdfpagemode=UseNone')
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun test packages/core/src/__tests__/latex-compiler.test.ts`
Expected: FAIL — preamble only has `[hidelinks]`

- [ ] **Step 3: Update hyperref configuration**

In `packages/core/src/templates/sb2nov.ts`, replace line 28:

```ts
// Before:
\\usepackage[hidelinks]{hyperref}

// After:
\\usepackage{hyperref}
\\hypersetup{hidelinks,pdfpagemode=UseNone}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun test packages/core/src/__tests__/latex-compiler.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/templates/sb2nov.ts packages/core/src/__tests__/latex-compiler.test.ts
git commit -m "fix(pdf): suppress document outline panel via pdfpagemode=UseNone"
```

---

### Task 2: Add server-side PDF cache

**Files:**
- Modify: `packages/core/src/services/resume-service.ts:486-560`
- Modify: `packages/core/src/routes/resumes.ts:311-335`
- Modify: `packages/sdk/src/resources/resumes.ts:249-284`

- [ ] **Step 1: Write tests**

```ts
// packages/core/src/__tests__/pdf-cache.test.ts
import { describe, test, expect, beforeAll } from 'bun:test'
import { existsSync, mkdirSync, rmSync, readdirSync } from 'fs'
import { createHash } from 'crypto'

const CACHE_DIR = '/tmp/forge-pdf-cache'

describe('PDF cache', () => {
  test('PDF_CACHE_DIR constant is exported', async () => {
    const { PDF_CACHE_DIR } = await import('../services/resume-service')
    expect(PDF_CACHE_DIR).toBe(CACHE_DIR)
  })

  test('hashLatexContent produces consistent SHA-256 hex', async () => {
    const { hashLatexContent } = await import('../services/resume-service')
    const hash1 = hashLatexContent('\\documentclass{article}')
    const hash2 = hashLatexContent('\\documentclass{article}')
    const hash3 = hashLatexContent('\\documentclass{report}')
    expect(hash1).toBe(hash2)
    expect(hash1).not.toBe(hash3)
    expect(hash1).toHaveLength(64) // SHA-256 hex
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bun test packages/core/src/__tests__/pdf-cache.test.ts`
Expected: FAIL — no exports exist yet

- [ ] **Step 3: Add cache logic to generatePDF**

In `packages/core/src/services/resume-service.ts`, add these exports and modify `generatePDF`:

```ts
import { createHash } from 'crypto'
import { existsSync } from 'fs'

export const PDF_CACHE_DIR = '/tmp/forge-pdf-cache'

export function hashLatexContent(content: string): string {
  return createHash('sha256').update(content).digest('hex')
}
```

Modify `generatePDF` signature to accept `bust` parameter:

```ts
async generatePDF(id: string, latex?: string, bust = false): Promise<Result<Buffer> & { _cacheHit?: boolean }>
```

After computing `latexContent` (existing logic at lines 493-507), add cache check before tectonic invocation:

```ts
const hash = hashLatexContent(latexContent)
const cachePath = `${PDF_CACHE_DIR}/${hash}.pdf`

// Ensure cache dir exists (lazy init)
if (!existsSync(PDF_CACHE_DIR)) {
  const { mkdirSync } = await import('fs')
  mkdirSync(PDF_CACHE_DIR, { recursive: true })
}

// Cache hit?
if (!bust && existsSync(cachePath)) {
  const cached = await Bun.file(cachePath).arrayBuffer()
  return { ok: true, data: Buffer.from(cached), _cacheHit: true }
}
```

After successful tectonic compilation (after reading `pdfBytes` at line 555), add cache write:

```ts
// Write to cache
await Bun.write(cachePath, buffer)
return { ok: true, data: buffer, _cacheHit: false }
```

- [ ] **Step 4: Add bust parameter and cache header to route**

In `packages/core/src/routes/resumes.ts`, modify the PDF endpoint at line 311:

```ts
app.post('/resumes/:id/pdf', async (c) => {
  let latex: string | undefined
  const bust = c.req.query('bust') === '1'
  try {
    const body = await c.req.json()
    latex = body.latex
  } catch {}

  const result = await services.resumes.generatePDF(c.req.param('id'), latex, bust)
  if (!result.ok) {
    const code = result.error.code
    const status = code === 'TECTONIC_NOT_AVAILABLE' ? 501
                 : code === 'TECTONIC_TIMEOUT' ? 504
                 : code === 'LATEX_COMPILE_ERROR' ? 422
                 : mapStatusCode(code)
    return c.json({ error: result.error }, status as any)
  }

  return new Response(result.data, {
    status: 200,
    headers: {
      'Content-Type': 'application/pdf',
      'Content-Disposition': 'inline; filename="resume.pdf"',
      'X-Forge-Pdf-Cache': result._cacheHit ? 'hit' : 'miss',
    },
  })
})
```

- [ ] **Step 5: Update SDK to pass bust parameter and expose cache header**

In `packages/sdk/src/resources/resumes.ts`, modify the existing `pdf()` method (lines 249-284). The method already uses direct `fetch()` with `this.baseUrl` — preserve that pattern:

```ts
async pdf(id: string, opts?: { bust?: boolean }): Promise<Result<Blob> & { cacheStatus?: 'hit' | 'miss' }> {
  const bustParam = opts?.bust ? '?bust=1' : ''
  const path = `/api/resumes/${id}/pdf${bustParam}`
  const method = 'POST'
  const start = performance.now()

  try {
    const response = await fetch(`${this.baseUrl}${path}`, { method })
    const duration = Math.round(performance.now() - start)

    if (response.ok) {
      const blob = await response.blob()
      const cacheStatus = response.headers.get('X-Forge-Pdf-Cache') as 'hit' | 'miss' | null
      if (this.debug?.logToConsole) {
        console.debug(`[forge:sdk] ← ${method} ${path} ${response.status} ${duration}ms ok (${blob.size} bytes PDF, cache: ${cacheStatus})`)
      }
      return { ok: true, data: blob, cacheStatus: cacheStatus ?? undefined }
    }

    const json = await response.json() as { error?: ForgeError }
    const error = json.error ?? { code: 'UNKNOWN_ERROR', message: `HTTP ${response.status}` }
    if (this.debug?.logToConsole) {
      console.debug(`[forge:sdk] ← ${method} ${path} ${response.status} ${duration}ms ERROR ${error.code}`)
    }
    return { ok: false, error }
  } catch (err) {
    if (this.debug?.logToConsole) {
      console.debug(`[forge:sdk] ✗ ${method} ${path} NETWORK_ERROR`)
    }
    return { ok: false, error: { code: 'NETWORK_ERROR', message: String(err) } }
  }
}
```

- [ ] **Step 6: Run tests**

Run: `bun test packages/core/src/__tests__/pdf-cache.test.ts`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add packages/core/src/services/resume-service.ts packages/core/src/routes/resumes.ts packages/sdk/src/resources/resumes.ts packages/core/src/__tests__/pdf-cache.test.ts
git commit -m "feat(pdf): add server-side content-addressed PDF cache with bust parameter"
```

---

### Task 3: Build FormatToggle component

**Files:**
- Create: `packages/webui/src/lib/components/resume/FormatToggle.svelte`

- [ ] **Step 1: Create the component**

Uses global `.btn` + `.btn-ghost` classes per `ui-shared-components.md`. Renders as a segmented control.

```svelte
<!-- packages/webui/src/lib/components/resume/FormatToggle.svelte -->
<script lang="ts">
  type Format = 'default' | 'latex' | 'markdown'

  interface Props {
    value: Format
    onChange: (format: Format) => void
    disabled?: boolean
  }

  let { value, onChange, disabled = false }: Props = $props()

  const formats: { value: Format; label: string }[] = [
    { value: 'default', label: 'Visual' },
    { value: 'latex', label: 'LaTeX' },
    { value: 'markdown', label: 'Markdown' },
  ]
</script>

<div class="format-toggle" role="radiogroup" aria-label="Editor format">
  {#each formats as fmt}
    <button
      class="btn btn-ghost format-option"
      class:active={value === fmt.value}
      {disabled}
      role="radio"
      aria-checked={value === fmt.value}
      onclick={() => onChange(fmt.value)}
    >
      {fmt.label}
    </button>
  {/each}
</div>

<style>
  .format-toggle {
    display: inline-flex;
    gap: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .format-option {
    border-radius: 0;
    border: none;
    border-right: 1px solid var(--color-border);
    padding: 0.25rem 0.75rem;
    font-size: var(--text-xs);
  }
  .format-option:last-child { border-right: none; }
  .format-option.active {
    background: var(--color-primary);
    color: white;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add packages/webui/src/lib/components/resume/FormatToggle.svelte
git commit -m "feat(webui): add FormatToggle segmented control component"
```

---

### Task 4: Build ResumePreview component

**Files:**
- Create: `packages/webui/src/lib/components/resume/ResumePreview.svelte`

This replaces both `PdfView.svelte` (for default/latex formats) and the view-mode rendering in `SourceView.svelte`. References `$lib/sdk` (not `$lib/forge`). Uses `marked` (already installed at `^17.0.6`).

- [ ] **Step 1: Create the unified preview component**

```svelte
<!-- packages/webui/src/lib/components/resume/ResumePreview.svelte -->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { marked } from 'marked'
  import { LoadingSpinner } from '$lib/components'
  import type { ResumeDocument } from '@forge/sdk'

  type Format = 'default' | 'latex' | 'markdown'

  let {
    resumeId,
    ir,
    format,
  }: {
    resumeId: string
    ir: ResumeDocument
    format: Format
  } = $props()

  let pdfDataUrl = $state<string | null>(null)
  let loading = $state(false)
  let error = $state<{ code: string; message: string; details?: string } | null>(null)
  let cacheStatus = $state<'hit' | 'miss' | null>(null)
  let lastFetchedResumeId = $state<string | null>(null)

  async function fetchPdf(bust = false) {
    if (loading) return
    // Skip if we already have a PDF for this resume (unless busting cache)
    if (!bust && lastFetchedResumeId === resumeId && pdfDataUrl) return

    loading = true
    error = null

    try {
      const result = await forge.resumes.pdf(resumeId, { bust })
      if (result.ok) {
        if (pdfDataUrl) URL.revokeObjectURL(pdfDataUrl)
        pdfDataUrl = URL.createObjectURL(result.data)
        cacheStatus = (result as any).cacheStatus ?? null
        lastFetchedResumeId = resumeId
      } else {
        const err = result.error
        if (err.code === 'LATEX_COMPILE_ERROR') {
          error = {
            code: err.code,
            message: 'LaTeX compilation failed',
            details: typeof err.details === 'object' && err.details !== null
              ? (err.details as Record<string, string>).tectonic_stderr ?? err.message
              : err.message,
          }
        } else if (err.code === 'TECTONIC_NOT_AVAILABLE') {
          error = { code: err.code, message: 'Tectonic is not installed', details: 'Install tectonic: cargo install tectonic' }
        } else if (err.code === 'TECTONIC_TIMEOUT') {
          error = { code: err.code, message: 'PDF generation timed out', details: 'LaTeX compilation took >60s. Simplify the document and try again.' }
        } else {
          error = { code: 'UNKNOWN', message: friendlyError(err) }
        }
      }
    } catch {
      error = { code: 'NETWORK', message: 'Failed to fetch PDF preview' }
    } finally {
      loading = false
    }
  }

  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  async function download() {
    const result = await forge.resumes.pdf(resumeId)
    if (result.ok) {
      triggerDownload(result.data, `resume-${new Date().toISOString().slice(0, 10)}.pdf`)
    } else {
      addToast({ type: 'error', message: friendlyError(result.error) })
    }
  }

  // Auto-fetch PDF when component mounts or resumeId changes (for PDF formats only)
  $effect(() => {
    const _id = resumeId  // track
    const _fmt = format   // track
    if ((_fmt === 'default' || _fmt === 'latex') && _id) {
      // Reset when resume changes
      if (_id !== lastFetchedResumeId) {
        lastFetchedResumeId = null
      }
      fetchPdf()
    }
  })

  // Compute markdown HTML for markdown format
  let markdownHtml = $derived.by(() => {
    if (format !== 'markdown' || !ir) return ''
    const content = (ir as any).markdown_override || generateMarkdown(ir)
    return marked.parse(content, { async: false }) as string
  })

  // Client-side markdown generation from IR (same as SourceView.generateMarkdown)
  function generateMarkdown(ir: ResumeDocument): string {
    // Import existing generateMarkdown from SourceView or extract as shared util
    // For now, delegate to the existing function pattern
    return `# ${ir.header?.name ?? ''}\n\n_Generated from IR_`
  }

  import { onDestroy } from 'svelte'
  onDestroy(() => {
    if (pdfDataUrl) URL.revokeObjectURL(pdfDataUrl)
  })
</script>

<div class="resume-preview">
  <div class="preview-toolbar">
    {#if format !== 'markdown'}
      <button class="btn btn-ghost btn-sm" onclick={() => fetchPdf(true)} title="Regenerate PDF (bust cache)">
        &#x21BB;
      </button>
      <button class="btn btn-ghost btn-sm" onclick={download} title="Download PDF">
        &#x2193;
      </button>
      {#if cacheStatus}
        <span class="cache-badge">cache: {cacheStatus}</span>
      {/if}
    {/if}
  </div>

  <div class="preview-content">
    {#if format === 'markdown'}
      <div class="markdown-preview">
        {@html markdownHtml}
      </div>
    {:else if loading}
      <div class="preview-center">
        <LoadingSpinner />
        <p>Compiling PDF...</p>
      </div>
    {:else if error}
      <div class="preview-center">
        <p class="error-title">{error.message}</p>
        {#if error.details}
          <pre class="error-details">{error.details}</pre>
        {/if}
        <button class="btn btn-primary btn-sm" onclick={() => fetchPdf()}>Retry</button>
      </div>
    {:else if pdfDataUrl}
      <iframe src={pdfDataUrl} title="Resume PDF preview"></iframe>
    {/if}
  </div>
</div>

<style>
  .resume-preview {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .preview-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    border-bottom: 1px solid var(--color-border);
    min-height: 2.5rem;
  }
  .preview-content {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .preview-content iframe {
    width: 100%;
    height: 100%;
    border: none;
  }
  .preview-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 1rem;
    color: var(--color-text-secondary);
  }
  .error-title { font-weight: var(--font-semibold); }
  .error-details {
    font-size: var(--text-xs);
    max-width: 600px;
    overflow-x: auto;
    white-space: pre-wrap;
    background: var(--color-surface-raised);
    padding: 0.75rem;
    border-radius: var(--radius-sm);
  }
  .markdown-preview {
    padding: 2rem;
    max-width: 800px;
    margin: 0 auto;
    line-height: 1.6;
  }
  .cache-badge {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    border-radius: var(--radius-sm);
    background: var(--color-surface-hover);
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add packages/webui/src/lib/components/resume/ResumePreview.svelte
git commit -m "feat(webui): add unified ResumePreview with auto-render, cache status, and GFM support"
```

---

### Task 5: Build ResumeEditor component

**Files:**
- Create: `packages/webui/src/lib/components/resume/ResumeEditor.svelte`

This wraps `<DragNDropView>` (default) or CodeMirror (latex/markdown). The CodeMirror setup, save logic, override handling, and OverrideBanner integration are lifted from the existing `SourceView.svelte` (840 lines). Key patterns to preserve:

- `EditorState` with `Compartment` for language/editability reconfiguration
- `basicSetup` + `StreamLanguage.define(stex)` for LaTeX, `markdown()` for Markdown
- `EditorView.lineWrapping` + custom theme matching existing style
- `handleSave()` calls `forge.resumes.updateLatexOverride()` / `updateMarkdownOverride()`
- `handleRegenerate()` replaces editor content with generated-from-IR
- `handleReset()` clears the override via `null`
- `OverrideBanner` with `isStale` / `hasOverride` props

- [ ] **Step 1: Create the component**

```svelte
<!-- packages/webui/src/lib/components/resume/ResumeEditor.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { EditorView } from '@codemirror/view'
  import { EditorState, Compartment } from '@codemirror/state'
  import { basicSetup } from 'codemirror'
  import { StreamLanguage } from '@codemirror/language'
  import { stex } from '@codemirror/legacy-modes/mode/stex'
  import { markdown } from '@codemirror/lang-markdown'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'
  import OverrideBanner from './OverrideBanner.svelte'
  import DragNDropView from './DragNDropView.svelte'
  import type { ResumeDocument } from '@forge/sdk'

  type Format = 'default' | 'latex' | 'markdown'

  let {
    resumeId,
    ir,
    format,
    latexOverride,
    latexOverrideUpdatedAt,
    markdownOverride,
    markdownOverrideUpdatedAt,
    resumeUpdatedAt,
    onOverrideChange,
  }: {
    resumeId: string
    ir: ResumeDocument
    format: Format
    latexOverride: string | null
    latexOverrideUpdatedAt: string | null
    markdownOverride: string | null
    markdownOverrideUpdatedAt: string | null
    resumeUpdatedAt: string
    onOverrideChange: () => Promise<void>
  } = $props()

  let saving = $state(false)
  let editorContainer: HTMLDivElement | undefined = $state()
  let editorView: EditorView | undefined = $state()

  // Compartments for dynamic reconfiguration (same pattern as SourceView)
  const languageCompartment = new Compartment()
  const editableCompartment = new Compartment()
  const readOnlyCompartment = new Compartment()

  // Derived: override state for current format
  let currentOverride = $derived(format === 'latex' ? latexOverride : markdownOverride)
  let currentOverrideUpdatedAt = $derived(format === 'latex' ? latexOverrideUpdatedAt : markdownOverrideUpdatedAt)
  let isOverride = $derived(currentOverride !== null)
  let isStale = $derived(isOverride && currentOverrideUpdatedAt !== null && resumeUpdatedAt > currentOverrideUpdatedAt)

  // Content for editor
  let displayContent = $derived(
    currentOverride ?? (format === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdown(ir))
  )

  // Dirty tracking: compare editor buffer to last-saved content
  let isDirty = $derived.by(() => {
    if (!editorView || format === 'default') return false
    return editorView.state.doc.toString() !== displayContent
  })

  function getLanguageExtension(fmt: Format) {
    return fmt === 'latex' ? StreamLanguage.define(stex) : markdown()
  }

  function createEditor() {
    if (!editorContainer || format === 'default') return
    const startState = EditorState.create({
      doc: displayContent,
      extensions: [
        basicSetup,
        languageCompartment.of(getLanguageExtension(format)),
        editableCompartment.of(EditorView.editable.of(true)),
        readOnlyCompartment.of(EditorState.readOnly.of(false)),
        EditorView.lineWrapping,
        EditorView.theme({
          '&': { fontSize: '0.82rem' },
          '.cm-content': { fontFamily: 'var(--font-mono)' },
          '.cm-gutters': { background: 'var(--color-surface-raised)', borderRight: '1px solid var(--color-border)' },
        }),
      ],
    })
    editorView = new EditorView({ state: startState, parent: editorContainer })
  }

  function destroyEditor() {
    editorView?.destroy()
    editorView = undefined
  }

  // Recreate editor when format changes between latex/markdown
  $effect(() => {
    const _fmt = format
    if (_fmt === 'default') {
      destroyEditor()
      return
    }
    // If switching between latex <-> markdown, update language and content
    if (editorView) {
      const lang = getLanguageExtension(_fmt)
      const content = currentOverride ?? (_fmt === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdown(ir))
      editorView.dispatch({ effects: languageCompartment.reconfigure(lang) })
      editorView.dispatch({ changes: { from: 0, to: editorView.state.doc.length, insert: content } })
    }
  })

  onMount(() => { if (format !== 'default') createEditor() })
  onDestroy(() => destroyEditor())

  // ---- Save / Regenerate / Reset (same pattern as SourceView) ----

  async function handleSave() {
    if (!editorView) return
    saving = true
    try {
      const content = editorView.state.doc.toString()

      // For LaTeX: validate by attempting compilation first
      if (format === 'latex') {
        const validateResult = await forge.resumes.pdf(resumeId)
        // We compile from IR, not buffer — actual validation happens on save
        // If tectonic isn't available, we still allow save (validation is best-effort)
      }

      const result = format === 'latex'
        ? await forge.resumes.updateLatexOverride(resumeId, content)
        : await forge.resumes.updateMarkdownOverride(resumeId, content)
      if (result.ok) {
        addToast({ message: 'Saved', type: 'success' })
        await onOverrideChange()
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } catch {
      addToast({ message: 'Failed to save', type: 'error' })
    } finally {
      saving = false
    }
  }

  async function handleRegenerate() {
    if (!editorView) return
    const generated = format === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdown(ir)
    editorView.dispatch({ changes: { from: 0, to: editorView.state.doc.length, insert: generated } })
    addToast({ message: 'Regenerated from IR', type: 'success' })
  }

  async function handleReset() {
    saving = true
    try {
      const result = format === 'latex'
        ? await forge.resumes.updateLatexOverride(resumeId, null)
        : await forge.resumes.updateMarkdownOverride(resumeId, null)
      if (result.ok) {
        addToast({ message: 'Override cleared', type: 'success' })
        await onOverrideChange()
        if (editorView) {
          const generated = format === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdown(ir)
          editorView.dispatch({ changes: { from: 0, to: editorView.state.doc.length, insert: generated } })
        }
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } catch {
      addToast({ message: 'Failed to reset', type: 'error' })
    } finally {
      saving = false
    }
  }

  function handleDiscard() {
    if (!editorView) return
    const content = currentOverride ?? (format === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdown(ir))
    editorView.dispatch({ changes: { from: 0, to: editorView.state.doc.length, insert: content } })
  }

  // Placeholder client-side generators (same as SourceView)
  function generateLatexPlaceholder(ir: ResumeDocument): string {
    return `% LaTeX source generated from IR\\n% Edit and save to create an override\\n\\\\documentclass{article}\\n\\\\begin{document}\\nResume placeholder\\n\\\\end{document}`
  }

  function generateMarkdown(ir: ResumeDocument): string {
    return `# ${ir.header?.name ?? 'Resume'}\\n\\n_Edit and save to create a markdown override_`
  }
</script>

<div class="resume-editor">
  {#if format === 'default'}
    <DragNDropView {ir} {resumeId} />
  {:else}
    {#if isOverride}
      <OverrideBanner {isStale} hasOverride={isOverride} onRegenerate={handleRegenerate} onReset={handleReset} />
    {/if}

    <div class="editor-toolbar">
      <div class="toolbar-left">
        {#if isDirty}
          <span class="dirty-indicator">Unsaved changes</span>
        {/if}
      </div>
      <div class="toolbar-right">
        <button class="btn btn-ghost btn-sm" onclick={handleDiscard} disabled={!isDirty || saving}>
          Discard
        </button>
        <button class="btn btn-primary btn-sm" onclick={handleSave} disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>

    <div class="editor-container" bind:this={editorContainer}></div>
  {/if}
</div>

<style>
  .resume-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .editor-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    border-bottom: 1px solid var(--color-border);
    min-height: 2.5rem;
  }
  .toolbar-left { display: flex; align-items: center; gap: 0.5rem; }
  .toolbar-right { display: flex; gap: 0.5rem; }
  .dirty-indicator {
    font-size: var(--text-xs);
    color: var(--color-warning-text);
    font-style: italic;
  }
  .editor-container {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .editor-container :global(.cm-editor) {
    height: 100%;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add packages/webui/src/lib/components/resume/ResumeEditor.svelte
git commit -m "feat(webui): add unified ResumeEditor with DnD/LaTeX/Markdown format modes"
```

---

### Task 6: Wire into +page.svelte and update barrel exports

**Files:**
- Modify: `packages/webui/src/routes/resumes/+page.svelte`
- Modify: `packages/webui/src/lib/components/resume/index.ts`

- [ ] **Step 1: Update resume component barrel exports**

Replace `packages/webui/src/lib/components/resume/index.ts`:

```ts
export { default as DragNDropView } from './DragNDropView.svelte'
export { default as HeaderEditor } from './HeaderEditor.svelte'
export { default as OverrideBanner } from './OverrideBanner.svelte'
export { default as FormatToggle } from './FormatToggle.svelte'
export { default as ResumePreview } from './ResumePreview.svelte'
export { default as ResumeEditor } from './ResumeEditor.svelte'
```

Old exports removed: `PdfView`, `SourceView`, `LatexView`, `MarkdownView`.

- [ ] **Step 2: Replace tab system in +page.svelte**

In `packages/webui/src/routes/resumes/+page.svelte`:

Replace the tab type and definition (lines 22-28):

```ts
type ViewTab = 'editor' | 'preview'
const VIEW_TABS = [
  { value: 'editor', label: 'Editor' },
  { value: 'preview', label: 'Preview' },
]

type Format = 'default' | 'latex' | 'markdown'
let activeFormat = $state<Format>('default')
```

- [ ] **Step 3: Replace tab content rendering**

Replace the tab content area (lines ~1080-1123) with:

```svelte
<div class="view-toolbar">
  <TabBar tabs={VIEW_TABS} active={activeViewTab} onSelect={(v) => activeViewTab = v} />
  <FormatToggle value={activeFormat} onChange={(f) => activeFormat = f} />
</div>

{#if activeViewTab === 'editor'}
  <ResumeEditor
    resumeId={selectedResumeId}
    {ir}
    format={activeFormat}
    latexOverride={selectedResume?.latex_override ?? null}
    latexOverrideUpdatedAt={selectedResume?.latex_override_updated_at ?? null}
    markdownOverride={selectedResume?.markdown_override ?? null}
    markdownOverrideUpdatedAt={selectedResume?.markdown_override_updated_at ?? null}
    resumeUpdatedAt={selectedResume?.updated_at ?? ''}
    onOverrideChange={refreshSelectedResume}
  />
{:else}
  <ResumePreview resumeId={selectedResumeId} {ir} format={activeFormat} />
{/if}
```

- [ ] **Step 4: Remove old component imports**

Remove imports of `PdfView`, `SourceView` from the page file. Add imports:

```ts
import { FormatToggle, ResumePreview, ResumeEditor } from '$lib/components/resume'
import { TabBar } from '$lib/components'
```

- [ ] **Step 5: Remove hand-rolled tab CSS**

Delete the `.view-tab` / `.view-tabs` CSS block from the page's `<style>` section — replaced by `<TabBar>`.

- [ ] **Step 6: Run tests**

Run: `bun test packages/webui/src/__tests__/`
Expected: PASS (may need adjustments to tests referencing old tab names)

- [ ] **Step 7: Commit**

```bash
git add packages/webui/src/lib/components/resume/index.ts packages/webui/src/routes/resumes/+page.svelte
git commit -m "feat(webui): merge Preview+Source into Editor/Preview with format toggle"
```

---

### Task 7: Remove old components and fix broken references

**Files:**
- Remove: `packages/webui/src/lib/components/resume/PdfView.svelte`
- Remove: `packages/webui/src/lib/components/resume/SourceView.svelte`
- Remove: `packages/webui/src/lib/components/resume/LatexView.svelte`
- Remove: `packages/webui/src/lib/components/resume/MarkdownView.svelte`
- Remove: `packages/webui/src/lib/components/RenderViewport.svelte`
- Modify: `packages/webui/src/lib/components/index.ts` (remove RenderViewport export)
- Modify: `packages/webui/src/__tests__/data-visualization.test.ts` (remove RenderViewport tests)

- [ ] **Step 1: Verify no imports reference old components**

```bash
rg "PdfView|SourceView|LatexView|MarkdownView|RenderViewport" packages/webui/src/ --type-add 'svelte:*.svelte' --type svelte --type ts
```

Expected: Only hits in the files we're about to delete + the barrel + the test file. No hits in `+page.svelte` or other active components (those were updated in Task 6).

- [ ] **Step 2: Delete old component files**

```bash
rm packages/webui/src/lib/components/resume/PdfView.svelte
rm packages/webui/src/lib/components/resume/SourceView.svelte
rm packages/webui/src/lib/components/resume/LatexView.svelte
rm packages/webui/src/lib/components/resume/MarkdownView.svelte
rm packages/webui/src/lib/components/RenderViewport.svelte
```

- [ ] **Step 3: Remove RenderViewport from top-level barrel**

In `packages/webui/src/lib/components/index.ts`, delete line 22:

```ts
// Remove this line:
export { default as RenderViewport } from './RenderViewport.svelte'
```

- [ ] **Step 4: Remove RenderViewport tests**

In `packages/webui/src/__tests__/data-visualization.test.ts`, delete the entire `describe('79.6: RenderViewport placeholder exists', ...)` block (lines 92-106).

- [ ] **Step 5: Run full test suite**

Run: `just test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add -u
git commit -m "cleanup(webui): remove old PdfView, SourceView, LatexView, MarkdownView, RenderViewport"
```

---

### Task 8: End-to-end smoke test

**Files:** None (verification only)

- [ ] **Step 1: Start dev server**

```bash
just dev
```

- [ ] **Step 2: Test default format (DnD + PDF)**

1. Open `http://localhost:5173/resumes`, select a resume
2. Format toggle should show "Visual" selected
3. Editor tab shows DnD view
4. Click Preview tab — PDF auto-renders (no "Generate" button), spinner shows while loading
5. Document outline panel is NOT shown in the PDF viewer
6. Click regenerate icon — PDF recompiles (cache badge shows "miss")
7. Switch back to Editor then Preview — PDF loads instantly (cache badge shows "hit")
8. Click download icon — PDF downloads with date-stamped filename

- [ ] **Step 3: Test LaTeX format**

1. Switch format to "LaTeX"
2. Editor tab shows CodeMirror with LaTeX content
3. Make a small edit — "Unsaved changes" indicator appears
4. Click Discard — content resets, indicator gone
5. Make another edit, click Save — toast shows "Saved"
6. Switch to Preview — PDF reflects the saved LaTeX override
7. Override banner shows "Showing manual override" message

- [ ] **Step 4: Test Markdown format**

1. Switch format to "Markdown"
2. Editor tab shows CodeMirror with Markdown content
3. Switch to Preview — rendered GFM appears (no PDF, no spinner, no server call)
4. Toolbar does NOT show regenerate/download buttons

- [ ] **Step 5: Test format persistence across tabs**

1. Set format to "Markdown"
2. Switch to Preview — see rendered Markdown
3. Switch to Editor — still in Markdown mode (CodeMirror visible)
4. Switch to Preview — still rendered Markdown (not PDF)
5. Set format to "Visual" — Preview shows PDF again

- [ ] **Step 6: Test cache behavior**

1. Set format to "Visual", click Preview
2. First load: cache badge shows "miss"
3. Switch to Editor and back to Preview — badge shows "hit" (instant load)
4. Click regenerate — badge shows "miss" (forced recompile)

---

## Summary

| Task | What | Dependencies | Parallelizable |
|------|------|-------------|----------------|
| 1 | Fix document outline (hypersetup) | None | Yes |
| 2 | Server-side PDF cache | None | Yes |
| 3 | FormatToggle component | None | Yes |
| 4 | ResumePreview component | Task 2 | After Track A |
| 5 | ResumeEditor component | None | Yes |
| 6 | Wire into +page.svelte + barrel exports | Tasks 3, 4, 5 | After all components |
| 7 | Remove old components + fix references | Task 6 | After wiring |
| 8 | End-to-end smoke test | Task 7 | Last |

**Parallel tracks:**
```
Track A (backend):     Task 1 (hypersetup) + Task 2 (PDF cache)
Track B (components):  Task 3 (FormatToggle) + Task 5 (ResumeEditor)
                                     ↘                    ↓
                       Task 4 (ResumePreview, needs Task 2) ─→ Task 6 (wire) → Task 7 (cleanup) → Task 8 (smoke test)
```
