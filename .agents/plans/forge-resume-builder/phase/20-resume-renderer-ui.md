# Phase 20: Resume Renderer UI

**Goal:** Build the four-mode resume preview UI (DragNDrop, Markdown, LaTeX, PDF) as tabs within the resume builder view, using the IR compiled by Phase 19's server-side compiler.

**Non-Goals:** Bi-directional LaTeX/Markdown parsing (one-way overrides only). Multiple LaTeX templates. Collaborative editing. Cloud LaTeX compilation.

**Depends on:** Phase 19 (IR compiler, format compilers, escape utilities, API endpoints for `/ir`, `/pdf`, `/header`, `/markdown-override`, `/latex-override` must all exist)
**Blocks:** Nothing (final phase)

**Parallelizable:** T20.3 (DragNDrop), T20.4 (Markdown), T20.5 (LaTeX), and T20.6 (PDF) are independent view components that can be built in parallel after T20.2 establishes the tab layout.

**Reference:** `refs/specs/2026-03-29-resume-renderer-and-entity-updates.md` sections 1.4 (View Modes) and 1.6 (Override Storage)

**Conventions (all components):**
- Svelte 5 syntax: `$state`, `$effect`, `$derived`, `$props`
- Import shared components from `$lib/components` (`StatusBadge`, `LoadingSpinner`, `EmptyState`, `ConfirmDialog`, `ToastContainer`, `DriftBanner`)
- Use `forge` SDK client from `$lib/sdk`
- Use `addToast` from `$lib/stores/toast.svelte` for notifications
- Use `friendlyError` from `$lib/sdk` for API error messages
- Types imported from `@forge/sdk` (including IR types: `ResumeDocument`, `ResumeSection`, `ExperienceBullet`, etc.)
- New components go in `packages/webui/src/lib/components/resume/`
- Re-export all resume components from `packages/webui/src/lib/components/resume/index.ts`

---

## Task 20.1: Install Dependencies

**Goal:** Add CodeMirror 6 and svelte-dnd-action to the webui package.

**Steps:**

1. Install all required packages:
   ```bash
   cd packages/webui && bun add svelte-dnd-action codemirror @codemirror/view @codemirror/state @codemirror/basic-setup @codemirror/lang-markdown @codemirror/language-data @codemirror/theme-one-dark
   ```

   Note: There is no official `@codemirror/lang-latex` package. LaTeX highlighting will use `@codemirror/legacy-modes` with the `stex` mode from CodeMirror 5:
   ```bash
   bun add @codemirror/language @codemirror/legacy-modes
   ```

2. Verify the build still passes:
   ```bash
   cd packages/webui && bun run build
   ```

3. Verify `svelte-dnd-action` types are available. If not, add `@types/svelte-dnd-action` or declare a module in `src/app.d.ts`.

**Package summary after install:**

| Package | Purpose |
|---------|---------|
| `svelte-dnd-action` | Drag-and-drop reordering for DragNDrop view |
| `codemirror` | Core editor (re-exports `@codemirror/view` + `@codemirror/state`) |
| `@codemirror/view` | EditorView API |
| `@codemirror/state` | EditorState API |
| `@codemirror/basic-setup` | Sensible defaults (line numbers, bracket matching, etc.) |
| `@codemirror/lang-markdown` | Markdown syntax highlighting |
| `@codemirror/language-data` | Language data for fenced code blocks in markdown preview |
| `@codemirror/language` | StreamLanguage adapter for legacy modes |
| `@codemirror/legacy-modes` | CodeMirror 5 modes including `stex` for LaTeX |
| `@codemirror/theme-one-dark` | Dark theme option for code editors |

**Acceptance Criteria:**
- [ ] All packages listed in `packages/webui/package.json` dependencies
- [ ] `bun run build` succeeds with no errors
- [ ] `bun run check` passes (Svelte type checking)

---

## Task 20.2: Resume Preview Layout (Tab Container)

**Goal:** Extend the resume builder view with a tabbed preview area below the existing resume header/sections. When a resume is selected, the lower half of the page shows DragNDrop | Markdown | LaTeX | PDF tabs.

**File:** `packages/webui/src/routes/resumes/+page.svelte`

**Changes to existing file:**

The existing `+page.svelte` has a two-panel layout: left panel (resume list/builder) and right panel (gap analysis). The change adds a tabbed preview area inside the left panel, below the sections listing. The sections listing becomes a collapsible "Structure" panel, and the preview tabs become the primary editing surface.

**New state variables (add to `<script>` block):**

```typescript
import type { ResumeDocument } from '@forge/sdk'
import DragNDropView from '$lib/components/resume/DragNDropView.svelte'
import MarkdownView from '$lib/components/resume/MarkdownView.svelte'
import LatexView from '$lib/components/resume/LatexView.svelte'
import PdfView from '$lib/components/resume/PdfView.svelte'
import HeaderEditor from '$lib/components/resume/HeaderEditor.svelte'

type ViewTab = 'dnd' | 'markdown' | 'latex' | 'pdf'

const VIEW_TABS: { value: ViewTab; label: string }[] = [
  { value: 'dnd', label: 'DragNDrop' },
  { value: 'markdown', label: 'Markdown' },
  { value: 'latex', label: 'LaTeX' },
  { value: 'pdf', label: 'PDF' },
]

let activeViewTab = $state<ViewTab>('dnd')
let ir = $state<ResumeDocument | null>(null)
let irLoading = $state(false)
let irError = $state<string | null>(null)
```

**New function — fetch IR:**

```typescript
async function loadIR(id: string) {
  irLoading = true
  irError = null
  try {
    const result = await forge.resumes.ir(id)
    if (result.ok) {
      ir = result.data
    } else {
      irError = friendlyError(result.error, 'Failed to load IR')
    }
  } catch (e) {
    irError = 'Failed to load resume IR'
  } finally {
    irLoading = false
  }
}
```

**Hook IR loading into the existing `selectedResumeId` effect:**

```typescript
$effect(() => {
  if (selectedResumeId) {
    loadResumeDetail(selectedResumeId)
    loadGapAnalysis(selectedResumeId)
    loadIR(selectedResumeId)       // <-- add this
  }
})
```

**New callback — handle IR updates from DragNDrop:**

```typescript
async function handleIRUpdate() {
  // After a DnD edit (reorder, clone, delete clone), refresh both IR and detail
  if (selectedResumeId) {
    await Promise.all([
      loadIR(selectedResumeId),
      loadResumeDetail(selectedResumeId),
    ])
  }
}
```

**Template addition (inside the `{:else if resumeDetail}` block, after the sections div):**

```svelte
<!-- View Mode Tabs -->
<div class="view-tabs-container">
  <div class="view-tabs">
    {#each VIEW_TABS as tab}
      <button
        class="view-tab"
        class:active={activeViewTab === tab.value}
        onclick={() => activeViewTab = tab.value}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <div class="view-content">
    {#if irLoading}
      <div class="loading-container">
        <LoadingSpinner message="Compiling resume..." />
      </div>
    {:else if irError}
      <div class="view-error">
        <p>{irError}</p>
        <button class="btn btn-secondary" onclick={() => selectedResumeId && loadIR(selectedResumeId)}>
          Retry
        </button>
      </div>
    {:else if ir}
      {#if activeViewTab === 'dnd'}
        <DragNDropView {ir} onUpdate={handleIRUpdate} resumeId={selectedResumeId!} />
      {:else if activeViewTab === 'markdown'}
        <MarkdownView
          {ir}
          override={resumeDetail.markdown_override ?? null}
          overrideUpdatedAt={resumeDetail.markdown_override_updated_at ?? null}
          resumeUpdatedAt={resumeDetail.updated_at}
          resumeId={selectedResumeId!}
          onOverrideChange={async () => { await loadResumeDetail(selectedResumeId!) }}
        />
      {:else if activeViewTab === 'latex'}
        <LatexView
          {ir}
          override={resumeDetail.latex_override ?? null}
          overrideUpdatedAt={resumeDetail.latex_override_updated_at ?? null}
          resumeUpdatedAt={resumeDetail.updated_at}
          resumeId={selectedResumeId!}
          onOverrideChange={async () => { await loadResumeDetail(selectedResumeId!) }}
        />
      {:else if activeViewTab === 'pdf'}
        <PdfView resumeId={selectedResumeId!} />
      {/if}
    {/if}
  </div>
</div>
```

**New styles (append to `<style>` block):**

```css
.view-tabs-container {
  margin-top: 1.5rem;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;
  background: #fff;
}

.view-tabs {
  display: flex;
  border-bottom: 1px solid #e5e7eb;
  background: #f9fafb;
}

.view-tab {
  padding: 0.75rem 1.25rem;
  border: none;
  background: transparent;
  font-size: 0.875rem;
  font-weight: 500;
  color: #6b7280;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: color 0.15s, border-color 0.15s;
  font-family: inherit;
}

.view-tab:hover {
  color: #374151;
}

.view-tab.active {
  color: #6c63ff;
  border-bottom-color: #6c63ff;
  background: #fff;
}

.view-content {
  min-height: 400px;
}

.view-error {
  padding: 2rem;
  text-align: center;
  color: #ef4444;
}
```

**SDK extension required (Phase 19 prerequisite):** The `ResumesResource` must expose `ir(id)`, `updateHeader(id, header)`, `updateMarkdownOverride(id, content)`, `updateLatexOverride(id, content)`, and `pdf(id)` methods. These are Phase 19 deliverables. For this phase plan, assume these exist:

```typescript
// Expected SDK methods (from Phase 19)
forge.resumes.ir(resumeId)                        // GET /api/resumes/:id/ir → Result<ResumeDocument>
forge.resumes.updateHeader(resumeId, header)      // PATCH /api/resumes/:id/header → Result<Resume>
forge.resumes.updateMarkdownOverride(resumeId, content) // PATCH /api/resumes/:id/markdown-override → Result<void>
forge.resumes.updateLatexOverride(resumeId, content)   // PATCH /api/resumes/:id/latex-override → Result<void>
forge.resumes.pdf(resumeId, opts?)                // POST /api/resumes/:id/pdf → Result<Blob>
```

**Acceptance Criteria:**
- [ ] Tab bar renders below the resume sections with four tabs
- [ ] Clicking a tab switches the active view
- [ ] DragNDrop is the default active tab
- [ ] IR is fetched on resume selection and passed to the active view component
- [ ] Loading spinner shown while IR compiles
- [ ] Error state with retry button when IR fetch fails
- [ ] Tab state resets when switching resumes

---

## Task 20.3: DragNDrop View Component

**Goal:** Build an interactive visual resume renderer with drag-to-reorder, inline editing, and provenance tooltips.

**File:** `packages/webui/src/lib/components/resume/DragNDropView.svelte`

**Props:**

```typescript
let {
  ir,
  resumeId,
  onUpdate,
}: {
  ir: ResumeDocument
  resumeId: string
  onUpdate: () => Promise<void>
} = $props()
```

**State:**

```typescript
import { dndzone } from 'svelte-dnd-action'
import { forge, friendlyError } from '$lib/sdk'
import { addToast } from '$lib/stores/toast.svelte'
import { LoadingSpinner } from '$lib/components'
import HeaderEditor from './HeaderEditor.svelte'
import type {
  ResumeDocument, ResumeSection, ExperienceGroup,
  ExperienceBullet, ExperienceSubheading
} from '@forge/sdk'

// Inline editing
let editingEntryId = $state<string | null>(null)
let editContent = $state('')
let editSaving = $state(false)

// Provenance tooltip
let tooltipEntry = $state<ExperienceBullet | null>(null)
let tooltipPosition = $state({ x: 0, y: 0 })

// DnD state per section (svelte-dnd-action requires items array with id)
let sectionItems = $state<Record<string, { id: string; items: any[] }>>({})
```

**Template structure:**

```svelte
<div class="dnd-view">
  <!-- Header -->
  <HeaderEditor header={ir.header} {resumeId} onSave={onUpdate} />

  <!-- Sections -->
  {#each ir.sections.sort((a, b) => a.display_order - b.display_order) as section (section.id)}
    <div class="dnd-section">
      <h3 class="dnd-section-title">{section.title}</h3>

      {#if section.type === 'experience'}
        {#each section.items as item}
          {#if item.kind === 'experience_group'}
            {@const group = item as ExperienceGroup}
            <div class="experience-group">
              <h4 class="org-name">{group.organization}</h4>
              {#each group.subheadings as sub (sub.id)}
                <div class="subheading">
                  <div class="subheading-header">
                    <span class="role-title">{sub.title}</span>
                    <span class="date-range">{sub.date_range}</span>
                  </div>
                  <div
                    class="bullet-list"
                    use:dndzone={{ items: sub.bullets, flipDurationMs: 200 }}
                    onconsider={(e) => handleDndConsider(section.id, sub.id, e)}
                    onfinalize={(e) => handleDndFinalize(section.id, sub.id, e)}
                  >
                    {#each sub.bullets as bullet (bullet.entry_id)}
                      <div
                        class="bullet-item"
                        class:cloned={bullet.is_cloned}
                        onmouseenter={(e) => showTooltip(bullet, e)}
                        onmouseleave={() => tooltipEntry = null}
                      >
                        {#if editingEntryId === bullet.entry_id}
                          <textarea
                            class="bullet-edit-textarea"
                            bind:value={editContent}
                            rows="3"
                          ></textarea>
                          <div class="bullet-edit-actions">
                            <button class="btn btn-sm btn-primary"
                                    onclick={() => saveEdit(bullet.entry_id!)}
                                    disabled={editSaving}>
                              {editSaving ? 'Saving...' : 'Save'}
                            </button>
                            <button class="btn btn-sm btn-ghost"
                                    onclick={() => editingEntryId = null}>
                              Cancel
                            </button>
                          </div>
                        {:else}
                          <span class="bullet-content">{bullet.content}</span>
                          <div class="bullet-actions">
                            {#if bullet.is_cloned}
                              <span class="clone-badge">Edited</span>
                              <button class="btn btn-xs btn-ghost"
                                      onclick={() => resetClone(bullet.entry_id!)}
                                      title="Reset to reference">
                                Reset
                              </button>
                            {/if}
                            <button class="btn btn-xs btn-ghost"
                                    onclick={() => startEdit(bullet)}>
                              Edit
                            </button>
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/each}

      {:else if section.type === 'skills'}
        {#each section.items as item}
          {#if item.kind === 'skill_group'}
            <div class="skill-categories">
              {#each item.categories as cat}
                <div class="skill-category">
                  <strong>{cat.label}:</strong> {cat.skills.join(', ')}
                </div>
              {/each}
            </div>
          {/if}
        {/each}

      {:else if section.type === 'education'}
        {#each section.items as item}
          {#if item.kind === 'education'}
            <div class="education-item">
              <div class="edu-header">
                <span class="edu-institution">{item.institution}</span>
                <span class="edu-date">{item.date}</span>
              </div>
              <span class="edu-degree">{item.degree}</span>
            </div>
          {/if}
        {/each}

      {:else if section.type === 'certifications'}
        {#each section.items as item}
          {#if item.kind === 'certification_group'}
            {#each item.categories as cat}
              <div class="cert-category">
                <strong>{cat.label}:</strong>
                {cat.certs.map(c => c.name).join(', ')}
              </div>
            {/each}
          {/if}
        {/each}

      {:else if section.type === 'projects'}
        {#each section.items as item}
          {#if item.kind === 'project'}
            <div class="project-item">
              <div class="project-header">
                <span class="project-name">{item.name}</span>
                {#if item.date}<span class="project-date">{item.date}</span>{/if}
              </div>
              {#each item.bullets as bullet (bullet.entry_id)}
                <div class="bullet-item" class:cloned={bullet.is_cloned}>
                  <span class="bullet-content">{bullet.content}</span>
                </div>
              {/each}
            </div>
          {/if}
        {/each}

      {:else if section.type === 'summary'}
        {#each section.items as item}
          {#if item.kind === 'summary'}
            <p class="summary-text">{item.content}</p>
          {/if}
        {/each}

      {:else}
        <!-- Generic fallback for clearance, presentations, awards, custom -->
        {#each section.items as item}
          <div class="generic-item">
            {#if 'content' in item}
              <p>{item.content}</p>
            {/if}
            {#if 'bullets' in item && Array.isArray(item.bullets)}
              {#each item.bullets as bullet}
                <div class="bullet-item">
                  <span class="bullet-content">{bullet.content}</span>
                </div>
              {/each}
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/each}
</div>

<!-- Provenance Tooltip -->
{#if tooltipEntry?.source_chain}
  <div class="provenance-tooltip" style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px;">
    <div class="tooltip-row"><strong>Source:</strong> {tooltipEntry.source_chain.source_id}</div>
    <div class="tooltip-row"><strong>Bullet:</strong> {tooltipEntry.source_chain.bullet_id}</div>
    <div class="tooltip-row"><strong>Perspective:</strong> {tooltipEntry.source_chain.perspective_id}</div>
    {#if tooltipEntry.is_cloned}
      <div class="tooltip-row tooltip-cloned">Content has been manually edited</div>
    {/if}
  </div>
{/if}
```

**Key functions:**

```typescript
function startEdit(bullet: ExperienceBullet) {
  editingEntryId = bullet.entry_id
  editContent = bullet.content
}

async function saveEdit(entryId: string) {
  editSaving = true
  try {
    const result = await forge.resumes.updateEntry(resumeId, entryId, {
      content: editContent,
    })
    if (result.ok) {
      addToast({ message: 'Entry updated (cloned)', type: 'success' })
      editingEntryId = null
      await onUpdate()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } finally {
    editSaving = false
  }
}

async function resetClone(entryId: string) {
  const result = await forge.resumes.updateEntry(resumeId, entryId, {
    content: null,
  })
  if (result.ok) {
    addToast({ message: 'Entry reset to reference', type: 'success' })
    await onUpdate()
  } else {
    addToast({ message: friendlyError(result.error), type: 'error' })
  }
}

function handleDndConsider(sectionId: string, subId: string, e: CustomEvent) {
  // Update local items for visual feedback during drag
  // svelte-dnd-action provides updated items array in e.detail.items
}

async function handleDndFinalize(sectionId: string, subId: string, e: CustomEvent) {
  // After drop: compute new positions, PATCH each moved entry
  const items = e.detail.items as ExperienceBullet[]
  for (let i = 0; i < items.length; i++) {
    if (items[i].entry_id) {
      await forge.resumes.updateEntry(resumeId, items[i].entry_id!, {
        position: i,
      })
    }
  }
  await onUpdate()
}

function showTooltip(bullet: ExperienceBullet, e: MouseEvent) {
  if (!bullet.source_chain) return
  tooltipEntry = bullet
  tooltipPosition = { x: e.clientX + 10, y: e.clientY + 10 }
}
```

**Styling notes:**
- Resume-like visual layout: centered, max-width 800px, serif font for content
- Drag handles (grip dots) on left of each bullet
- Cloned entries get a subtle left-border accent (amber)
- Hover reveals provenance tooltip positioned near cursor
- Edit mode replaces content with textarea inline

**Acceptance Criteria:**
- [ ] Renders all IR section types (experience, skills, education, projects, summary, certifications, clearance, presentations, custom)
- [ ] Experience entries grouped by organization with subheadings
- [ ] Bullet hover shows provenance tooltip with source chain
- [ ] Edit button enters inline edit mode with textarea
- [ ] Save edit calls `PATCH /api/resumes/:id/entries/:eid` with content
- [ ] Reset clone calls `PATCH /api/resumes/:id/entries/:eid` with `content: null`
- [ ] Drag-to-reorder within a subheading updates positions via PATCH
- [ ] `onUpdate` callback triggers IR and detail reload after every mutation

---

## Task 20.4: Markdown View Component

**Goal:** Build a split-pane Markdown editor with CodeMirror on the left and rendered HTML preview on the right, with override storage and staleness detection.

**File:** `packages/webui/src/lib/components/resume/MarkdownView.svelte`

**Props:**

```typescript
let {
  ir,
  override,
  overrideUpdatedAt,
  resumeUpdatedAt,
  resumeId,
  onOverrideChange,
}: {
  ir: ResumeDocument
  override: string | null
  overrideUpdatedAt: string | null
  resumeUpdatedAt: string
  resumeId: string
  onOverrideChange: () => Promise<void>
} = $props()
```

**State:**

```typescript
import { onMount, onDestroy } from 'svelte'
import { EditorView } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { basicSetup } from '@codemirror/basic-setup'
import { markdown } from '@codemirror/lang-markdown'
import { oneDark } from '@codemirror/theme-one-dark'
import { forge, friendlyError } from '$lib/sdk'
import { addToast } from '$lib/stores/toast.svelte'
import OverrideBanner from './OverrideBanner.svelte'
import type { ResumeDocument } from '@forge/sdk'

let editorContainer = $state<HTMLDivElement | null>(null)
let editorView = $state<EditorView | null>(null)
let previewHtml = $state('')
let editable = $state(false)
let saving = $state(false)

// Compute whether we're showing an override or generated content
let isOverride = $derived(override !== null)
let isStale = $derived(
  isOverride && overrideUpdatedAt !== null && resumeUpdatedAt > overrideUpdatedAt
)

// Content to display: override if exists, else compile from IR
let displayContent = $derived(override ?? compileMarkdown(ir))
```

**CodeMirror initialization (Svelte 5 pattern):**

CodeMirror 6 is imperative and does not work well with Svelte's reactivity. The correct pattern is to create the editor once on mount, then update its contents via `EditorView.dispatch()` when props change.

```typescript
onMount(() => {
  if (!editorContainer) return

  const state = EditorState.create({
    doc: displayContent,
    extensions: [
      basicSetup,
      markdown(),
      EditorView.editable.of(editable),
      EditorState.readOnly.of(!editable),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          previewHtml = renderMarkdownToHtml(update.state.doc.toString())
        }
      }),
      EditorView.theme({
        '&': { height: '100%', fontSize: '13px' },
        '.cm-scroller': { overflow: 'auto' },
        '.cm-content': { fontFamily: "'JetBrains Mono', 'Fira Code', monospace" },
      }),
    ],
  })

  editorView = new EditorView({
    state,
    parent: editorContainer,
  })

  // Initial preview render
  previewHtml = renderMarkdownToHtml(displayContent)
})

onDestroy(() => {
  editorView?.destroy()
})
```

**Reacting to prop changes:**

When the `override` or `ir` prop changes (e.g., after regenerate/reset), we need to update the editor content:

```typescript
$effect(() => {
  if (!editorView) return
  const currentDoc = editorView.state.doc.toString()
  if (currentDoc !== displayContent) {
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: displayContent,
      },
    })
    previewHtml = renderMarkdownToHtml(displayContent)
  }
})

// Update editability when toggled
$effect(() => {
  if (!editorView) return
  editorView.dispatch({
    effects: [
      // Reconfigure the editable and readOnly extensions
      // This requires using Compartments (see implementation detail below)
    ],
  })
})
```

**Compartments for dynamic reconfiguration:**

CodeMirror 6 uses `Compartment` for extensions that change at runtime (like editability):

```typescript
import { Compartment } from '@codemirror/state'

const editableCompartment = new Compartment()
const readOnlyCompartment = new Compartment()

// In EditorState.create extensions:
editableCompartment.of(EditorView.editable.of(false)),
readOnlyCompartment.of(EditorState.readOnly.of(true)),

// To toggle:
function toggleEditable(enable: boolean) {
  editable = enable
  editorView?.dispatch({
    effects: [
      editableCompartment.reconfigure(EditorView.editable.of(enable)),
      readOnlyCompartment.reconfigure(EditorState.readOnly.of(!enable)),
    ],
  })
}
```

**Markdown preview renderer:**

A simple function that converts markdown to HTML for the preview pane. Does NOT use a full markdown parser library -- uses a minimal regex-based renderer since the resume markdown structure is constrained:

```typescript
function renderMarkdownToHtml(md: string): string {
  return md
    .split('\n')
    .map(line => {
      if (line.startsWith('# '))       return `<h1>${esc(line.slice(2))}</h1>`
      if (line.startsWith('## '))      return `<h2>${esc(line.slice(3))}</h2>`
      if (line.startsWith('### '))     return `<h3>${esc(line.slice(4))}</h3>`
      if (line.startsWith('- '))       return `<li>${esc(line.slice(2))}</li>`
      if (line.startsWith('**') && line.includes(':**'))
        return `<p class="skill-line">${boldify(esc(line))}</p>`
      if (line.trim() === '')          return '<br>'
      return `<p>${boldify(esc(line))}</p>`
    })
    .join('\n')
}

function esc(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function boldify(s: string): string {
  return s.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
}
```

Note: If a full markdown parser is desired, add `marked` as a dependency and use `marked.parse(md)`. But for MVP, the constrained format means the simple renderer suffices. Evaluate during implementation.

**Markdown linter:**

```typescript
interface LintResult {
  valid: boolean
  errors: string[]
}

function lintMarkdown(content: string): LintResult {
  const errors: string[] = []
  const lines = content.split('\n')

  // Rule 1: Document begins with # Name (H1)
  const firstNonBlank = lines.find(l => l.trim() !== '')
  if (!firstNonBlank || !firstNonBlank.startsWith('# ')) {
    errors.push('Document must begin with a level-1 heading (# Name)')
  }

  // Rule 2: Sections start with ## (H2)
  const headings = lines.filter(l => /^#{1,6}\s/.test(l))
  const badHeadings = headings.filter(l =>
    !l.startsWith('# ') && !l.startsWith('## ') && !l.startsWith('### ')
  )
  if (badHeadings.length > 0) {
    errors.push('Only H1 (#), H2 (##), and H3 (###) headings are allowed')
  }

  // Rule 3: Bullet items start with "- " (not * or +)
  const badBullets = lines.filter(l => /^\s*[*+]\s/.test(l))
  if (badBullets.length > 0) {
    errors.push('Bullet items must start with "- " (not * or +)')
  }

  // Rule 4: No blank lines within a bullet item (heuristic: no "- " line followed by blank then non-"- " non-heading)
  // Simplified: warn if there are 3+ consecutive blank lines
  // Rule 5: No more than 2 consecutive blank lines
  let consecutiveBlanks = 0
  for (const line of lines) {
    if (line.trim() === '') {
      consecutiveBlanks++
      if (consecutiveBlanks > 2) {
        errors.push('No more than 2 consecutive blank lines allowed')
        break
      }
    } else {
      consecutiveBlanks = 0
    }
  }

  // Rule 6: Skills section items match **Label**: content pattern
  let inSkills = false
  for (const line of lines) {
    if (line.startsWith('## ')) {
      inSkills = line.toLowerCase().includes('skill')
    } else if (inSkills && line.startsWith('- ')) {
      if (!/^- \*\*.+\*\*:/.test(line)) {
        errors.push(`Skills section items must match "- **Label**: content" pattern`)
        break
      }
    }
  }

  return { valid: errors.length === 0, errors }
}
```

**Action functions:**

```typescript
function compileMarkdown(doc: ResumeDocument): string {
  // Calls the same pure function used server-side.
  // Imported from @forge/sdk or a shared utils package.
  // If not available client-side, fetch via API: GET /api/resumes/:id/ir?format=markdown
  // For now, assume a client-side compileToMarkdown exists in @forge/sdk
  return compileToMarkdown(doc)
}

async function handleSave() {
  if (!editorView) return
  const content = editorView.state.doc.toString()

  const lint = lintMarkdown(content)
  if (!lint.valid) {
    addToast({ message: `Lint errors: ${lint.errors.join('; ')}`, type: 'error', duration: 6000 })
    return
  }

  saving = true
  try {
    const result = await forge.resumes.updateMarkdownOverride(resumeId, content)
    if (result.ok) {
      addToast({ message: 'Markdown override saved', type: 'success' })
      await onOverrideChange()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } finally {
    saving = false
  }
}

async function handleRegenerate() {
  saving = true
  try {
    const freshContent = compileMarkdown(ir)
    const result = await forge.resumes.updateMarkdownOverride(resumeId, freshContent)
    if (result.ok) {
      addToast({ message: 'Markdown regenerated from IR', type: 'success' })
      await onOverrideChange()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } finally {
    saving = false
  }
}

async function handleReset() {
  saving = true
  try {
    const result = await forge.resumes.updateMarkdownOverride(resumeId, null)
    if (result.ok) {
      addToast({ message: 'Override cleared, showing generated content', type: 'success' })
      toggleEditable(false)
      await onOverrideChange()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } finally {
    saving = false
  }
}
```

**Template:**

```svelte
<div class="markdown-view">
  <!-- Override Banner -->
  <OverrideBanner
    {isStale}
    hasOverride={isOverride}
    onRegenerate={handleRegenerate}
    onReset={handleReset}
  />

  <!-- Toolbar -->
  <div class="editor-toolbar">
    {#if isOverride}
      <button class="btn btn-sm btn-primary" onclick={handleSave} disabled={saving}>
        {saving ? 'Saving...' : 'Save Override'}
      </button>
    {:else}
      <button class="btn btn-sm btn-secondary" onclick={() => {
        // Enable editing: first save the generated content as an override
        toggleEditable(true)
      }}>
        Enable Editing
      </button>
    {/if}
  </div>

  <!-- Split Pane -->
  <div class="split-pane">
    <div class="editor-pane" bind:this={editorContainer}></div>
    <div class="preview-pane">
      {@html previewHtml}
    </div>
  </div>
</div>
```

**Styling:**

```css
.markdown-view {
  display: flex;
  flex-direction: column;
  height: 600px;
}

.editor-toolbar {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid #e5e7eb;
  background: #f9fafb;
}

.split-pane {
  display: grid;
  grid-template-columns: 1fr 1fr;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.editor-pane {
  overflow: auto;
  border-right: 1px solid #e5e7eb;
}

.preview-pane {
  overflow-y: auto;
  padding: 1.5rem;
  font-family: Georgia, 'Times New Roman', serif;
  font-size: 0.9rem;
  line-height: 1.6;
}
```

**Acceptance Criteria:**
- [ ] CodeMirror editor renders with markdown syntax highlighting
- [ ] Split view shows editor on left, rendered preview on right
- [ ] When no override: editor shows generated markdown as read-only
- [ ] "Enable Editing" button makes editor writable
- [ ] When override exists: editor shows override, editable by default
- [ ] "Save Override" validates with linter, saves via PATCH on success
- [ ] Linter rejects: missing H1, wrong bullet syntax, excess blank lines, bad skills format
- [ ] Stale override banner shown when `resumeUpdatedAt > overrideUpdatedAt`
- [ ] "Regenerate" recompiles IR and saves as new override
- [ ] "Reset" sets override to null, returns to generated mode
- [ ] Preview updates live as user types in editor
- [ ] Editor destroyed on component unmount (no memory leaks)

---

## Task 20.5: LaTeX View Component

**Goal:** Build a split-pane LaTeX editor with CodeMirror (using legacy `stex` mode) on the left and a custom HTML preview on the right.

**File:** `packages/webui/src/lib/components/resume/LatexView.svelte`

**Props:** Same shape as MarkdownView:

```typescript
let {
  ir,
  override,
  overrideUpdatedAt,
  resumeUpdatedAt,
  resumeId,
  onOverrideChange,
}: {
  ir: ResumeDocument
  override: string | null
  overrideUpdatedAt: string | null
  resumeUpdatedAt: string
  resumeId: string
  onOverrideChange: () => Promise<void>
} = $props()
```

**State:** Same pattern as MarkdownView, with these differences:

```typescript
import { StreamLanguage } from '@codemirror/language'
import { stex } from '@codemirror/legacy-modes/mode/stex'

// Replace markdown() with StreamLanguage.define(stex) in editor extensions
```

**CodeMirror setup difference:**

```typescript
// In EditorState.create extensions array, replace:
//   markdown()
// with:
//   StreamLanguage.define(stex)
```

All other CodeMirror patterns (Compartments, mount/destroy, prop reactivity) are identical to T20.4.

**LaTeX preview renderer:**

A custom HTML renderer that maps the sb2nov resume commands to styled HTML. This is NOT a full LaTeX parser -- it handles only the known template commands:

```typescript
function renderLatexToHtml(latex: string): string {
  const lines = latex.split('\n')
  const output: string[] = []
  let inDocument = false
  let inItemList = false
  let inSubHeadingList = false

  for (const line of lines) {
    const trimmed = line.trim()

    // Skip preamble
    if (trimmed === '\\begin{document}') { inDocument = true; continue }
    if (trimmed === '\\end{document}') { inDocument = false; continue }
    if (!inDocument) continue

    // Section headers
    const sectionMatch = trimmed.match(/^\\section\{(.+?)\}/)
    if (sectionMatch) {
      if (inItemList) { output.push('</ul>'); inItemList = false }
      output.push(`<h2 class="latex-section">${esc(sectionMatch[1])}</h2>`)
      continue
    }

    // Subheading: \resumeSubheading{org}{}{role}{dates}
    const subMatch = trimmed.match(/^\\resumeSubheading\{(.+?)\}\{(.*?)\}\{(.+?)\}\{(.+?)\}/)
    if (subMatch) {
      output.push(`<div class="latex-subheading">`)
      output.push(`  <div class="latex-sub-row"><strong>${esc(subMatch[1])}</strong><span>${esc(subMatch[4])}</span></div>`)
      output.push(`  <div class="latex-sub-row"><em>${esc(subMatch[3])}</em><span>${esc(subMatch[2])}</span></div>`)
      output.push(`</div>`)
      continue
    }

    // Sub-sub heading: \resumeSubSubheading{role}{dates}
    const subsubMatch = trimmed.match(/^\\resumeSubSubheading\{(.+?)\}\{(.+?)\}/)
    if (subsubMatch) {
      output.push(`<div class="latex-subsubheading">`)
      output.push(`  <em>${esc(subsubMatch[1])}</em><span>${esc(subsubMatch[2])}</span>`)
      output.push(`</div>`)
      continue
    }

    // Project heading: \resumeProjectHeading{name}{date}
    const projMatch = trimmed.match(/^\\resumeProjectHeading\{(.+?)\}\{(.+?)\}/)
    if (projMatch) {
      output.push(`<div class="latex-project"><strong>${esc(projMatch[1])}</strong><span>${esc(projMatch[2])}</span></div>`)
      continue
    }

    // Item list markers
    if (trimmed === '\\resumeItemListStart') { output.push('<ul class="latex-items">'); inItemList = true; continue }
    if (trimmed === '\\resumeItemListEnd') { output.push('</ul>'); inItemList = false; continue }
    if (trimmed === '\\resumeSubHeadingListStart') { inSubHeadingList = true; continue }
    if (trimmed === '\\resumeSubHeadingListEnd') { inSubHeadingList = false; continue }

    // Items: \resumeItem{text}
    const itemMatch = trimmed.match(/^\\resumeItem\{(.+)\}/)
    if (itemMatch) {
      output.push(`<li>${esc(itemMatch[1])}</li>`)
      continue
    }

    // Skip empty and unknown lines
    if (trimmed && !trimmed.startsWith('%') && !trimmed.startsWith('\\')) {
      output.push(`<p>${esc(trimmed)}</p>`)
    }
  }

  return output.join('\n')
}
```

**LaTeX linter:**

```typescript
function lintLatex(content: string): LintResult {
  const errors: string[] = []

  // Rule 1: Must contain \begin{document} and \end{document}
  if (!content.includes('\\begin{document}')) {
    errors.push('Missing \\begin{document}')
  }
  if (!content.includes('\\end{document}')) {
    errors.push('Missing \\end{document}')
  }

  // Rule 2: Matched \resumeItemListStart / \resumeItemListEnd
  const itemStarts = (content.match(/\\resumeItemListStart/g) || []).length
  const itemEnds = (content.match(/\\resumeItemListEnd/g) || []).length
  if (itemStarts !== itemEnds) {
    errors.push(`Unmatched \\resumeItemListStart (${itemStarts}) / \\resumeItemListEnd (${itemEnds})`)
  }

  // Rule 3: Matched \resumeSubHeadingListStart / \resumeSubHeadingListEnd
  const subStarts = (content.match(/\\resumeSubHeadingListStart/g) || []).length
  const subEnds = (content.match(/\\resumeSubHeadingListEnd/g) || []).length
  if (subStarts !== subEnds) {
    errors.push(`Unmatched \\resumeSubHeadingListStart (${subStarts}) / \\resumeSubHeadingListEnd (${subEnds})`)
  }

  // Rule 4: Warn on unescaped & and % outside math mode (heuristic: outside $ delimiters)
  const lines = content.split('\n')
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    // Skip comment lines
    if (line.trim().startsWith('%')) continue
    // Check for unescaped & (not preceded by \)
    if (/(?<!\\)&/.test(line) && !line.includes('\\begin{tabular')) {
      // This is a warning, not a hard error
    }
  }

  // Rule 5: Security gate -- no \write18 or \input commands
  if (/\\write18/.test(content)) {
    errors.push('SECURITY: \\write18 is forbidden (enables shell escape)')
  }
  if (/\\input\{/.test(content)) {
    errors.push('SECURITY: \\input is forbidden (enables file inclusion)')
  }

  return { valid: errors.length === 0, errors }
}
```

**Action functions:** Same pattern as MarkdownView but calling `forge.resumes.updateLatexOverride()` instead of `updateMarkdownOverride()`.

**Template and styling:** Same split-pane layout as MarkdownView. The preview pane uses different CSS to approximate resume layout.

**Acceptance Criteria:**
- [ ] CodeMirror editor renders with LaTeX (stex) syntax highlighting
- [ ] Split view: editor left, custom HTML preview right
- [ ] LaTeX preview correctly renders `\section`, `\resumeSubheading`, `\resumeItem`, `\resumeProjectHeading`
- [ ] Override/staleness logic identical to Markdown view
- [ ] LaTeX linter validates: document delimiters, matched list markers, security gate
- [ ] `\write18` and `\input` are rejected as security violations
- [ ] Save/Regenerate/Reset work against `/latex-override` endpoint

---

## Task 20.6: PDF View Component

**Goal:** Build a PDF generation and display component.

**File:** `packages/webui/src/lib/components/resume/PdfView.svelte`

**Props:**

```typescript
let {
  resumeId,
}: {
  resumeId: string
} = $props()
```

**State:**

```typescript
import { forge, friendlyError } from '$lib/sdk'
import { addToast } from '$lib/stores/toast.svelte'
import { LoadingSpinner } from '$lib/components'

let pdfDataUrl = $state<string | null>(null)
let loading = $state(false)
let error = $state<{ code: string; message: string; details?: string } | null>(null)
```

**Functions:**

```typescript
async function generatePdf() {
  loading = true
  error = null
  pdfDataUrl = null

  try {
    const result = await forge.resumes.pdf(resumeId)
    if (result.ok) {
      // result.data is a Blob
      const blob = result.data as Blob
      pdfDataUrl = URL.createObjectURL(blob)
      addToast({ message: 'PDF generated', type: 'success' })
    } else {
      const err = result.error
      if (err.code === 'LATEX_COMPILE_ERROR') {
        error = {
          code: err.code,
          message: 'LaTeX compilation failed',
          details: err.details?.tectonic_stderr ?? err.message,
        }
      } else if (err.code === 'TECTONIC_NOT_AVAILABLE') {
        error = {
          code: err.code,
          message: 'Tectonic is not installed',
          details: 'Install tectonic for PDF generation: cargo install tectonic',
        }
      } else if (err.code === 'TECTONIC_TIMEOUT') {
        error = {
          code: err.code,
          message: 'PDF generation timed out',
          details: 'The LaTeX compilation took too long (>60s). Simplify the document and try again.',
        }
      } else {
        error = { code: 'UNKNOWN', message: friendlyError(err) }
      }
    }
  } catch (e) {
    error = { code: 'NETWORK', message: 'Failed to generate PDF' }
  } finally {
    loading = false
  }
}

// Clean up object URL on unmount
import { onDestroy } from 'svelte'
onDestroy(() => {
  if (pdfDataUrl) URL.revokeObjectURL(pdfDataUrl)
})
```

**Template:**

```svelte
<div class="pdf-view">
  {#if !pdfDataUrl && !loading && !error}
    <!-- Initial state: show generate button -->
    <div class="pdf-empty">
      <p class="pdf-empty-text">Click "Generate PDF" to compile your resume to PDF using tectonic.</p>
      <button class="btn btn-primary" onclick={generatePdf}>
        Generate PDF
      </button>
    </div>

  {:else if loading}
    <div class="pdf-loading">
      <LoadingSpinner size="lg" message="Compiling PDF..." />
      <p class="pdf-loading-sub">This may take 10-30 seconds on first run (tectonic downloads packages).</p>
    </div>

  {:else if error}
    <div class="pdf-error">
      <div class="pdf-error-header">
        <span class="pdf-error-code">{error.code}</span>
        <span class="pdf-error-message">{error.message}</span>
      </div>
      {#if error.details}
        <pre class="pdf-error-details">{error.details}</pre>
      {/if}
      <button class="btn btn-secondary" onclick={generatePdf}>
        Retry
      </button>
    </div>

  {:else if pdfDataUrl}
    <div class="pdf-toolbar">
      <button class="btn btn-secondary btn-sm" onclick={generatePdf}>
        Regenerate
      </button>
      <a class="btn btn-primary btn-sm" href={pdfDataUrl} download="resume.pdf">
        Download PDF
      </a>
    </div>
    <iframe
      class="pdf-frame"
      src={pdfDataUrl}
      title="Resume PDF Preview"
    ></iframe>
  {/if}
</div>
```

**Styling:**

```css
.pdf-view {
  display: flex;
  flex-direction: column;
  height: 600px;
}

.pdf-empty, .pdf-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  gap: 1rem;
  padding: 2rem;
  text-align: center;
  color: #6b7280;
}

.pdf-loading-sub {
  font-size: 0.8rem;
  color: #9ca3af;
}

.pdf-error {
  padding: 1.5rem;
}

.pdf-error-header {
  display: flex;
  gap: 0.75rem;
  align-items: baseline;
  margin-bottom: 0.75rem;
}

.pdf-error-code {
  padding: 0.2rem 0.5rem;
  background: #fef2f2;
  color: #ef4444;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  font-family: monospace;
}

.pdf-error-message {
  font-weight: 500;
  color: #374151;
}

.pdf-error-details {
  background: #1a1a2e;
  color: #e0e0e0;
  padding: 1rem;
  border-radius: 6px;
  font-size: 0.8rem;
  overflow-x: auto;
  max-height: 300px;
  overflow-y: auto;
  margin-bottom: 1rem;
  white-space: pre-wrap;
}

.pdf-toolbar {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid #e5e7eb;
  background: #f9fafb;
}

.pdf-frame {
  flex: 1;
  width: 100%;
  border: none;
}
```

**Acceptance Criteria:**
- [ ] Initial state shows "Generate PDF" button
- [ ] Loading state shows spinner with first-run note
- [ ] On success: PDF displayed in iframe, download link available
- [ ] On 422 (compile error): shows error code + tectonic stderr in pre block
- [ ] On 501 (not installed): shows install instructions
- [ ] On 504 (timeout): shows timeout message
- [ ] "Regenerate" button fetches a fresh PDF
- [ ] "Download PDF" provides a download link
- [ ] Object URL cleaned up on component destroy

---

## Task 20.7: Resume Header Editor

**Goal:** Build an inline form for editing resume header metadata (name, contact info, clearance).

**File:** `packages/webui/src/lib/components/resume/HeaderEditor.svelte`

**Props:**

```typescript
import type { ResumeHeader } from '@forge/sdk'

let {
  header,
  resumeId,
  onSave,
}: {
  header: ResumeHeader
  resumeId: string
  onSave: () => Promise<void>
} = $props()
```

**State:**

```typescript
import { forge, friendlyError } from '$lib/sdk'
import { addToast } from '$lib/stores/toast.svelte'

let editing = $state(false)
let saving = $state(false)
let form = $state<ResumeHeader>({ ...header })
```

**Template:**

```svelte
<div class="header-editor">
  {#if editing}
    <form class="header-form" onsubmit={(e) => { e.preventDefault(); handleSave() }}>
      <div class="header-form-grid">
        <div class="form-field">
          <label for="hdr-name">Name</label>
          <input id="hdr-name" type="text" bind:value={form.name} required />
        </div>
        <div class="form-field">
          <label for="hdr-tagline">Tagline</label>
          <input id="hdr-tagline" type="text" bind:value={form.tagline}
                 placeholder="Security Engineer | Cloud + DevSecOps" />
        </div>
        <div class="form-field">
          <label for="hdr-location">Location</label>
          <input id="hdr-location" type="text" bind:value={form.location}
                 placeholder="Reston, VA" />
        </div>
        <div class="form-field">
          <label for="hdr-email">Email</label>
          <input id="hdr-email" type="email" bind:value={form.email} />
        </div>
        <div class="form-field">
          <label for="hdr-phone">Phone</label>
          <input id="hdr-phone" type="tel" bind:value={form.phone} />
        </div>
        <div class="form-field">
          <label for="hdr-linkedin">LinkedIn</label>
          <input id="hdr-linkedin" type="url" bind:value={form.linkedin} />
        </div>
        <div class="form-field">
          <label for="hdr-github">GitHub</label>
          <input id="hdr-github" type="url" bind:value={form.github} />
        </div>
        <div class="form-field">
          <label for="hdr-website">Website</label>
          <input id="hdr-website" type="url" bind:value={form.website} />
        </div>
        <div class="form-field full-width">
          <label for="hdr-clearance">Clearance</label>
          <input id="hdr-clearance" type="text" bind:value={form.clearance}
                 placeholder="TS/SCI with CI Polygraph - Active" />
        </div>
      </div>
      <div class="header-form-actions">
        <button class="btn btn-ghost" type="button" onclick={() => { editing = false; form = { ...header } }}>
          Cancel
        </button>
        <button class="btn btn-primary" type="submit" disabled={saving}>
          {saving ? 'Saving...' : 'Save Header'}
        </button>
      </div>
    </form>

  {:else}
    <div class="header-display">
      <div class="header-display-top">
        <h2 class="header-name">{header.name}</h2>
        <button class="btn btn-sm btn-ghost" onclick={() => { editing = true; form = { ...header } }}>
          Edit Header
        </button>
      </div>
      {#if header.tagline}
        <p class="header-tagline">{header.tagline}</p>
      {/if}
      <div class="header-contact">
        {#if header.location}<span>{header.location}</span>{/if}
        {#if header.email}<span>{header.email}</span>{/if}
        {#if header.phone}<span>{header.phone}</span>{/if}
        {#if header.linkedin}<a href={header.linkedin} target="_blank">LinkedIn</a>{/if}
        {#if header.github}<a href={header.github} target="_blank">GitHub</a>{/if}
        {#if header.website}<a href={header.website} target="_blank">Website</a>{/if}
      </div>
      {#if header.clearance}
        <p class="header-clearance">{header.clearance}</p>
      {/if}
    </div>
  {/if}
</div>
```

**Save function:**

```typescript
async function handleSave() {
  saving = true
  try {
    const result = await forge.resumes.updateHeader(resumeId, form)
    if (result.ok) {
      addToast({ message: 'Header updated', type: 'success' })
      editing = false
      await onSave()
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
  } finally {
    saving = false
  }
}
```

**Styling:**

```css
.header-display {
  padding: 1.25rem;
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  margin-bottom: 1rem;
  text-align: center;
}

.header-display-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.header-name {
  font-size: 1.5rem;
  font-weight: 700;
  color: #1a1a2e;
  flex: 1;
  text-align: center;
}

.header-tagline {
  font-size: 0.95rem;
  color: #374151;
  margin-top: 0.25rem;
}

.header-contact {
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
  margin-top: 0.5rem;
  font-size: 0.85rem;
  color: #6b7280;
}

.header-contact a {
  color: #6c63ff;
  text-decoration: none;
}

.header-clearance {
  margin-top: 0.5rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: #059669;
}

.header-form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.full-width {
  grid-column: 1 / -1;
}

.header-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 1rem;
}
```

**Acceptance Criteria:**
- [ ] Display mode shows name, tagline, contact info, clearance in resume-header style
- [ ] "Edit Header" button switches to form mode
- [ ] Form has fields for all 9 header properties
- [ ] Save calls `PATCH /api/resumes/:id/header`
- [ ] Cancel reverts form to current header values
- [ ] Header updates reflected in IR on next fetch

---

## Task 20.8: Stale Override Banner Component

**Goal:** A reusable banner component for Markdown and LaTeX views that indicates override status.

**File:** `packages/webui/src/lib/components/resume/OverrideBanner.svelte`

**Props:**

```typescript
let {
  isStale,
  hasOverride,
  onRegenerate,
  onReset,
}: {
  isStale: boolean
  hasOverride: boolean
  onRegenerate: () => void
  onReset: () => void
} = $props()
```

**Template:**

```svelte
{#if hasOverride}
  <div class="override-banner" class:stale={isStale} class:current={!isStale}>
    {#if isStale}
      <div class="banner-message">
        <span class="banner-icon">&#x26A0;</span>
        <span>The structured resume has been updated since this override was saved. The text below may not reflect recent changes.</span>
      </div>
      <div class="banner-actions">
        <button class="banner-btn banner-regenerate" onclick={onRegenerate}>
          Regenerate
        </button>
        <button class="banner-btn banner-reset" onclick={onReset}>
          Reset to Generated
        </button>
      </div>
    {:else}
      <div class="banner-message">
        <span class="banner-icon-info">&#x2139;</span>
        <span>Showing manual override. Edits here will not update the structured resume data.</span>
      </div>
      <div class="banner-actions">
        <button class="banner-btn banner-reset" onclick={onReset}>
          Reset to Generated
        </button>
      </div>
    {/if}
  </div>
{/if}
```

**Styling:**

```css
.override-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.6rem 0.75rem;
  font-size: 0.8rem;
  border-bottom: 1px solid;
}

.override-banner.stale {
  background: #fffbeb;
  border-color: #fde68a;
  color: #92400e;
}

.override-banner.current {
  background: #eff6ff;
  border-color: #bfdbfe;
  color: #1e40af;
}

.banner-message {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.banner-icon {
  font-size: 1rem;
  flex-shrink: 0;
}

.banner-icon-info {
  font-size: 1rem;
  flex-shrink: 0;
}

.banner-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
}

.banner-btn {
  padding: 0.25rem 0.5rem;
  border: 1px solid;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 500;
  cursor: pointer;
  font-family: inherit;
  transition: background 0.1s;
}

.banner-regenerate {
  background: #6c63ff;
  border-color: #6c63ff;
  color: #fff;
}

.banner-regenerate:hover {
  background: #5a52e0;
}

.banner-reset {
  background: transparent;
  border-color: #d1d5db;
  color: #6b7280;
}

.banner-reset:hover {
  background: #f3f4f6;
}
```

**Acceptance Criteria:**
- [ ] Hidden when no override exists (`hasOverride === false`)
- [ ] Amber banner when stale (`resumeUpdatedAt > overrideUpdatedAt`)
- [ ] Blue/neutral banner when override is current
- [ ] "Regenerate" button visible only when stale
- [ ] "Reset to Generated" button always visible when override exists

---

## Task 20.9: Component Barrel Export

**Goal:** Create barrel export for resume components and register them in the shared components index.

**File:** `packages/webui/src/lib/components/resume/index.ts`

```typescript
export { default as DragNDropView } from './DragNDropView.svelte'
export { default as MarkdownView } from './MarkdownView.svelte'
export { default as LatexView } from './LatexView.svelte'
export { default as PdfView } from './PdfView.svelte'
export { default as HeaderEditor } from './HeaderEditor.svelte'
export { default as OverrideBanner } from './OverrideBanner.svelte'
```

Do NOT add these to the top-level `$lib/components/index.ts` -- they are domain-specific and should be imported from `$lib/components/resume` directly.

**Acceptance Criteria:**
- [ ] Barrel file exports all 6 resume components
- [ ] Import paths in `+page.svelte` use `$lib/components/resume/ComponentName.svelte` or `$lib/components/resume`
- [ ] `bun run check` passes

---

## Task 20.10: Update Navigation

**Goal:** Add `/domains` to the navigation sidebar if not already present (from Phase 17 domain CRUD).

**File:** `packages/webui/src/routes/+layout.svelte`

**Change:** Add the Domains entry to `navItems` if missing:

```typescript
const navItems = [
  { href: '/', label: 'Dashboard' },
  { href: '/sources', label: 'Sources' },
  { href: '/bullets', label: 'Bullets' },
  { href: '/resumes', label: 'Resumes' },
  { href: '/organizations', label: 'Organizations' },
  { href: '/domains', label: 'Domains' },         // <-- add if missing
  { href: '/skills', label: 'Skills' },
  { href: '/archetypes', label: 'Archetypes' },
  { href: '/chain', label: 'Chain View' },
  { href: '/logs', label: 'Logs' },
  { href: '/notes', label: 'Notes' },
]
```

**Acceptance Criteria:**
- [ ] `/domains` link appears in sidebar between Organizations and Skills
- [ ] All existing nav links still work
- [ ] Active state highlights correctly for `/domains`

---

## Task 20.11: Integration and E2E Tests

**Goal:** Verify the full resume renderer pipeline works end-to-end.

**Note:** These are manual verification steps and/or Playwright E2E tests if the test infrastructure exists. If Playwright is not set up, document as manual test procedures.

**Test scenarios:**

1. **IR round-trip:**
   - Create a resume via the UI
   - Add entries from the perspective picker
   - Switch to DragNDrop tab -- verify IR renders with correct sections and bullets
   - Verify experience entries are grouped by organization

2. **DragNDrop editing:**
   - Click "Edit" on a bullet -- verify inline textarea appears
   - Type new content, click "Save" -- verify toast "Entry updated (cloned)"
   - Verify the bullet now shows "Edited" badge
   - Click "Reset" -- verify returns to reference content

3. **DragNDrop reorder:**
   - Drag a bullet to a new position within the same subheading
   - Verify the bullet order persists after page reload

4. **Markdown flow:**
   - Switch to Markdown tab -- verify generated markdown appears read-only
   - Click "Enable Editing" -- verify editor becomes writable
   - Make an edit, click "Save Override" -- verify toast "Markdown override saved"
   - Switch to another tab and back -- verify override persists
   - Edit structured data via DnD tab, switch back to Markdown -- verify stale banner appears
   - Click "Regenerate" -- verify fresh markdown replaces stale override
   - Click "Reset to Generated" -- verify override cleared, editor returns to read-only

5. **Markdown linter:**
   - Remove the H1 heading, try to save -- verify error toast
   - Change a bullet from `- ` to `* `, try to save -- verify error toast
   - Add 4 consecutive blank lines, try to save -- verify error toast

6. **LaTeX flow:**
   - Same as Markdown flow but with LaTeX tab
   - Verify LaTeX syntax highlighting in editor
   - Verify preview renders `\resumeSubheading` as styled HTML

7. **LaTeX security gate:**
   - Add `\write18{ls}` to the LaTeX, try to save -- verify error toast about security
   - Add `\input{/etc/passwd}` -- verify error toast about security

8. **PDF generation:**
   - Switch to PDF tab, click "Generate PDF"
   - If tectonic installed: verify PDF displays in iframe
   - If tectonic not installed: verify 501 error with install instructions
   - Click "Download PDF" -- verify file downloads

9. **Header editing:**
   - In DragNDrop view, click "Edit Header"
   - Change name and tagline, save
   - Switch to Markdown tab -- verify header reflects changes (after regenerate if override exists)

10. **Override staleness:**
    - Save a Markdown override
    - Edit structured data (add an entry via DnD)
    - Switch to Markdown tab -- verify amber stale banner appears
    - Click "Regenerate" -- verify banner changes to blue/neutral

**Acceptance Criteria:**
- [ ] All 10 test scenarios pass (manual or automated)
- [ ] No console errors during normal operation
- [ ] No memory leaks (CodeMirror destroyed on unmount, object URLs revoked)

---

## Task 20.12: Documentation

**Goal:** Update project docs with the resume renderer feature.

**File:** `docs/src/webui/views.md` (add section)

**Content to add:**

```markdown
### Resume Renderer

The resume builder includes a four-mode preview system:

| Tab | Purpose | Editable | Storage |
|-----|---------|----------|---------|
| DragNDrop | Visual resume layout | Inline edit + drag reorder | Updates IR via API |
| Markdown | GHFM split editor | Override (one-way) | `markdown_override` column |
| LaTeX | LaTeX split editor | Override (one-way) | `latex_override` column |
| PDF | Compiled PDF viewer | Read-only | Generated on demand via tectonic |

Override staleness: When structured data changes after an override is saved, an amber banner warns the user. "Regenerate" recompiles from IR; "Reset" clears the override.
```

**File:** `docs/src/api/routes.md` (add endpoints)

```markdown
### Resume Renderer Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/resumes/:id/ir` | Compile and return IR |
| PATCH | `/api/resumes/:id/header` | Update resume header |
| PATCH | `/api/resumes/:id/markdown-override` | Save/reset markdown override |
| PATCH | `/api/resumes/:id/latex-override` | Save/reset LaTeX override |
| POST | `/api/resumes/:id/pdf` | Generate PDF via tectonic |
```

**Acceptance Criteria:**
- [ ] Views doc updated with renderer tab table
- [ ] Routes doc updated with 5 new endpoints

---

## Implementation Order

```
T20.1  Install deps
  |
T20.9  Barrel export (empty file, populated as components land)
  |
T20.8  OverrideBanner (no deps, used by T20.4 + T20.5)
  |
T20.7  HeaderEditor (no deps, used by T20.3)
  |
  +---> T20.3  DragNDropView (needs HeaderEditor, svelte-dnd-action)
  |
  +---> T20.4  MarkdownView (needs OverrideBanner, CodeMirror)
  |
  +---> T20.5  LatexView (needs OverrideBanner, CodeMirror)
  |
  +---> T20.6  PdfView (standalone)
  |
  +---- all four merge into ---->
  |
T20.2  Resume Preview Layout (imports all four, wires into +page.svelte)
  |
T20.10 Navigation update
  |
T20.11 Integration tests
  |
T20.12 Documentation
```

Note: T20.2 is listed early in the task list for readability, but the actual `+page.svelte` rewrite should happen AFTER the four view components exist, since it imports them. Alternatively, implement T20.2 first with placeholder `<p>TODO</p>` divs for each tab, then fill in as components land.

---

## File Inventory

| File | Action | Task |
|------|--------|------|
| `packages/webui/package.json` | Modify (add deps) | T20.1 |
| `packages/webui/src/lib/components/resume/index.ts` | Create | T20.9 |
| `packages/webui/src/lib/components/resume/OverrideBanner.svelte` | Create | T20.8 |
| `packages/webui/src/lib/components/resume/HeaderEditor.svelte` | Create | T20.7 |
| `packages/webui/src/lib/components/resume/DragNDropView.svelte` | Create | T20.3 |
| `packages/webui/src/lib/components/resume/MarkdownView.svelte` | Create | T20.4 |
| `packages/webui/src/lib/components/resume/LatexView.svelte` | Create | T20.5 |
| `packages/webui/src/lib/components/resume/PdfView.svelte` | Create | T20.6 |
| `packages/webui/src/routes/resumes/+page.svelte` | Modify (add tabs + IR) | T20.2 |
| `packages/webui/src/routes/+layout.svelte` | Modify (add /domains) | T20.10 |
| `docs/src/webui/views.md` | Modify | T20.12 |
| `docs/src/api/routes.md` | Modify | T20.12 |

**Total new files:** 8
**Total modified files:** 4
