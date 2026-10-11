<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte'
  import { EditorView } from '@codemirror/view'
  import { EditorState, Compartment } from '@codemirror/state'
  import { basicSetup } from 'codemirror'
  import { StreamLanguage } from '@codemirror/language'
  import { stex } from '@codemirror/legacy-modes/mode/stex'
  import { markdown } from '@codemirror/lang-markdown'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { generateMarkdownFromIR } from '$lib/resume-markdown'
  import OverrideBanner from './OverrideBanner.svelte'
  import DragNDropView from './DragNDropView.svelte'
  import type { ResumeDocument } from '@forge/sdk'

  type Format = 'default' | 'latex' | 'markdown'

  let {
    ir,
    resumeId,
    format,
    latexOverride,
    latexOverrideUpdatedAt,
    markdownOverride,
    markdownOverrideUpdatedAt,
    resumeUpdatedAt,
    onOverrideChange,
    onUpdate,
    onAddEntry,
    onAddSection,
    onDeleteSection,
    onRenameSection,
    onMoveSection,
    onRemoveEntry,
    onRemoveCertification,
    onUpdateSummary,
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
    // DragNDropView passthrough props
    onUpdate: () => Promise<void>
    onAddEntry?: (sectionId: string, entryType: string, sourceId?: string, sourceLabel?: string) => void
    onAddSection?: (entryType: string, title: string) => void
    onDeleteSection?: (sectionId: string) => void
    onRenameSection?: (sectionId: string, newTitle: string) => void
    onMoveSection?: (sectionId: string, direction: 'up' | 'down') => void
    onRemoveEntry?: (entryId: string) => Promise<void>
    onRemoveCertification?: (rcId: string) => Promise<void>
    onUpdateSummary?: (update: {
      summary_id?: string | null
      summary_override?: string | null
    }) => Promise<void>
  } = $props()

  let saving = $state(false)

  let editorContainer: HTMLDivElement | undefined = $state()
  let editorView: EditorView | undefined = $state()

  // Compartments for dynamic reconfiguration
  const languageCompartment = new Compartment()

  // Derived: current override and timestamps based on format
  let currentOverride = $derived(
    format === 'latex' ? latexOverride : markdownOverride
  )
  let currentOverrideUpdatedAt = $derived(
    format === 'latex' ? latexOverrideUpdatedAt : markdownOverrideUpdatedAt
  )
  let isOverride = $derived(currentOverride !== null)
  let isStale = $derived(
    isOverride && currentOverrideUpdatedAt !== null && resumeUpdatedAt > currentOverrideUpdatedAt
  )

  // Content for display: override takes priority, otherwise generate from IR
  let displayContent = $derived(
    currentOverride ?? (format === 'latex' ? generateLatexPlaceholder(ir) : generateMarkdownFromIR(ir))
  )

  // Dirty tracking: compare current editor buffer to displayContent
  let isDirty = $state(false)

  function getLanguageExtension(fmt: Format) {
    if (fmt === 'latex') return StreamLanguage.define(stex)
    return markdown()
  }

  function createEditor() {
    if (!editorContainer) return

    const startState = EditorState.create({
      doc: displayContent,
      extensions: [
        basicSetup,
        languageCompartment.of(getLanguageExtension(format)),
        EditorView.lineWrapping,
        EditorView.theme({
          '&': { fontSize: '0.82rem' },
          '.cm-content': { fontFamily: 'var(--font-mono)' },
          '.cm-gutters': {
            background: 'var(--color-surface-raised)',
            borderRight: '1px solid var(--color-border)',
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            isDirty = editorView?.state.doc.toString() !== displayContent
          }
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

  // Create, destroy, or reconfigure editor when format changes
  $effect(() => {
    const _format = format  // track reactivity

    if (_format === 'default') {
      destroyEditor()
      return
    }

    // If no editor exists yet, create it after DOM renders
    if (!editorView) {
      // Wait for Svelte to flush the DOM so editorContainer is mounted
      tick().then(() => {
        if (!editorView && format !== 'default') {
          createEditor()
        }
      })
      return
    }

    // Editor exists — reconfigure language and content
    const lang = getLanguageExtension(_format)
    const content = _format === 'latex'
      ? (latexOverride ?? generateLatexPlaceholder(ir))
      : (markdownOverride ?? generateMarkdownFromIR(ir))

    editorView.dispatch({
      effects: languageCompartment.reconfigure(lang),
    })
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: content,
      },
    })
    isDirty = false
  })

  onMount(() => {
    if (format !== 'default') {
      createEditor()
    }
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
      const result = format === 'latex'
        ? await forge.resumes.updateLatexOverride(resumeId, content)
        : await forge.resumes.updateMarkdownOverride(resumeId, content)
      if (result.ok) {
        addToast({ message: 'Saved', type: 'success' })
        isDirty = false
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

  async function handleDiscard() {
    if (!editorView) return
    const content = format === 'latex'
      ? (latexOverride ?? generateLatexPlaceholder(ir))
      : (markdownOverride ?? generateMarkdownFromIR(ir))
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: content,
      },
    })
    isDirty = false
  }

  async function handleRegenerate() {
    if (!editorView) return
    saving = true
    try {
      const generated = format === 'latex'
        ? generateLatexPlaceholder(ir)
        : generateMarkdownFromIR(ir)
      editorView.dispatch({
        changes: {
          from: 0,
          to: editorView.state.doc.length,
          insert: generated,
        },
      })
      isDirty = editorView.state.doc.toString() !== displayContent
      addToast({ message: 'Regenerated from IR', type: 'success' })
    } finally {
      saving = false
    }
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
          const generated = format === 'latex'
            ? generateLatexPlaceholder(ir)
            : generateMarkdownFromIR(ir)
          editorView.dispatch({
            changes: {
              from: 0,
              to: editorView.state.doc.length,
              insert: generated,
            },
          })
        }
        isDirty = false
      } else {
        addToast({ message: friendlyError(result.error), type: 'error' })
      }
    } catch {
      addToast({ message: 'Failed to reset', type: 'error' })
    } finally {
      saving = false
    }
  }

  // ---- Content generators (ported from SourceView.svelte) ----

  function escLatex(text: string): string {
    return text
      .replace(/\\/g, '\\textbackslash{}')
      .replace(/[&%$#_{}~^]/g, (c) => `\\${c}`)
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
    if (doc.header.clearance) lines.push(`\\textbf{${escLatex(doc.header.clearance)}}`)
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
            lines.push(`\\textbf{${escLatex(cat.label)}:} ${cat.certs.map(c => escLatex(c.in_progress ? `${c.name} (in progress)` : c.name)).join(', ')}`)
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
      lines.push('')
    }

    lines.push('\\end{document}')
    return lines.join('\n')
  }

</script>

{#if format === 'default'}
  <DragNDropView
    {ir}
    {resumeId}
    {onUpdate}
    {onAddEntry}
    {onAddSection}
    {onDeleteSection}
    {onRenameSection}
    {onMoveSection}
    {onRemoveEntry}
    {onRemoveCertification}
    {onUpdateSummary}
  />
{:else}
  <div class="resume-editor">
    <!-- Override banner -->
    <OverrideBanner
      {isStale}
      hasOverride={isOverride}
      onRegenerate={handleRegenerate}
      onReset={handleReset}
    />

    <!-- Toolbar: dirty indicator + save/discard -->
    <div class="editor-toolbar">
      <span class="format-label">{format === 'latex' ? 'LaTeX' : 'Markdown'} source</span>

      {#if isDirty}
        <span class="dirty-indicator">Unsaved changes</span>
      {/if}

      <div class="action-group">
        {#if isDirty}
          <button class="action-btn" onclick={handleDiscard} disabled={saving}>
            Discard
          </button>
          <button class="action-btn save" onclick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        {/if}
        {#if isOverride && !isDirty}
          <button class="action-btn danger" onclick={handleReset} disabled={saving}>
            Reset to Generated
          </button>
        {/if}
      </div>
    </div>

    <!-- CodeMirror editor container -->
    <div class="editor-wrapper" bind:this={editorContainer}></div>
  </div>
{/if}

<style>
  .resume-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface-raised);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .format-label {
    font-size: var(--text-xs);
    font-weight: var(--font-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .dirty-indicator {
    font-size: var(--text-xs);
    color: var(--color-warning-text);
    background: var(--color-warning-subtle);
    padding: 0.15rem 0.45rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-warning-border);
  }

  .action-group {
    display: flex;
    gap: 0.35rem;
    margin-left: auto;
  }

  .action-btn {
    padding: 0.3rem 0.6rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--text-secondary);
    font-size: var(--text-xs);
    cursor: pointer;
    transition: all 0.12s;
    font-family: inherit;
  }

  .action-btn:hover {
    background: var(--color-surface-sunken);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn.save {
    background: var(--color-primary);
    color: var(--text-inverse);
    border-color: var(--color-primary);
  }

  .action-btn.save:hover {
    background: var(--color-primary-hover);
  }

  .action-btn.danger {
    color: var(--color-danger-text);
    border-color: var(--color-danger-subtle);
  }

  .action-btn.danger:hover {
    background: var(--color-danger-subtle);
  }

  .editor-wrapper {
    flex: 1;
    overflow: auto;
  }

  .editor-wrapper :global(.cm-editor) {
    height: 100%;
  }
</style>
