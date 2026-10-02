<script lang="ts">
  import { onDestroy } from 'svelte'
  import { marked } from 'marked'
  import { forge, forgeMode, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { generateMarkdownFromIR } from '$lib/resume-markdown'
  import { LoadingSpinner } from '$lib/components'

  import type { ResumeDocument } from '@forge/sdk'

  interface Props {
    resumeId: string
    ir: ResumeDocument
    format: 'default' | 'latex' | 'markdown'
  }

  let { resumeId, ir, format }: Props = $props()

  let pdfDataUrl = $state<string | null>(null)
  let loading = $state(false)
  let error = $state<{ code: string; message: string; details?: string } | null>(null)
  let cacheStatus = $state<'hit' | 'miss' | undefined>(undefined)
  // Set when the server could not honour something about the resume (a LaTeX override).
  let notice = $state<string | null>(null)
  let lastFetchedResumeId = $state<string | null>(null)

  // Auto-fetch PDF on mount and when resumeId/format changes
  $effect(() => {
    const _resumeId = resumeId
    const _format = format

    // Reset tracking when resumeId changes so a new fetch is triggered
    if (lastFetchedResumeId !== null && lastFetchedResumeId !== _resumeId) {
      lastFetchedResumeId = null
    }

    if (_format === 'default' || _format === 'latex') {
      // Guard against redundant fetches
      if (lastFetchedResumeId === _resumeId) return
      fetchPdf(false)
    }
  })

  async function fetchPdf(bust: boolean) {
    loading = true
    error = null
    cacheStatus = undefined
    notice = null

    // Revoke previous URL if any
    if (pdfDataUrl) {
      URL.revokeObjectURL(pdfDataUrl)
      pdfDataUrl = null
    }

    try {
      const result = await forge.resumes.pdf(resumeId, { bust })
      if (result.ok) {
        const blob = result.data as Blob
        pdfDataUrl = URL.createObjectURL(blob)
        cacheStatus = result.cacheStatus
        notice = result.notice ?? null
        lastFetchedResumeId = resumeId
      } else {
        const err = result.error
        if (err.code === 'TYPST_COMPILE_ERROR') {
          const details = (err as { details?: unknown }).details
          error = {
            code: err.code,
            message: 'The resume did not compile to a PDF',
            details: Array.isArray(details) ? details.join('\n') : err.message,
          }
        } else if (err.code === 'NOT_IMPLEMENTED' || err.code === 'PDF_UNAVAILABLE') {
          error = { code: err.code, message: 'PDF output is not available', details: err.message }
        } else {
          error = { code: 'UNKNOWN', message: friendlyError(err) }
        }
      }
    } catch (e) {
      error = { code: 'NETWORK', message: 'Failed to fetch PDF' }
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

  async function handleDownload() {
    try {
      const result = await forge.resumes.pdf(resumeId)
      if (result.ok) {
        triggerDownload(result.data as Blob, 'resume.pdf')
      } else {
        addToast({ type: 'error', message: friendlyError(result.error) })
      }
    } catch (e) {
      addToast({ type: 'error', message: 'Failed to download PDF' })
    }
  }

  function getMarkdownHtml(): string {
    const content = (ir as any).markdown_override ?? generateMarkdownFromIR(ir)
    return marked.parse(content, { async: false }) as string
  }

  onDestroy(() => {
    if (pdfDataUrl) URL.revokeObjectURL(pdfDataUrl)
  })
</script>

<div class="resume-preview">
  {#if format !== 'markdown'}
    <div class="preview-toolbar">
      <button
        class="btn btn-ghost btn-sm"
        onclick={() => fetchPdf(true)}
        disabled={loading}
      >
        Regenerate
      </button>
      <button
        class="btn btn-primary btn-sm"
        onclick={handleDownload}
        disabled={loading || !!error}
      >
        Download PDF
      </button>
      {#if cacheStatus}
        <span class="cache-badge cache-badge--{cacheStatus}">{cacheStatus === 'hit' ? 'cached' : 'fresh'}</span>
      {/if}
    </div>
  {/if}

  <div class="preview-content">
    {#if format === 'markdown'}
      <div class="markdown-preview">
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html getMarkdownHtml()}
      </div>
    {:else if loading}
      <div class="preview-loading">
        <LoadingSpinner size="lg" message="Compiling PDF..." />
        {#if forgeMode === 'wasm'}
          <p class="preview-loading-sub">The first PDF in this browser downloads the PDF compiler (about 9 MB, once).</p>
        {/if}
      </div>
    {:else if error}
      <div class="preview-error">
        <div class="preview-error-header">
          <span class="preview-error-code">{error.code}</span>
          <span class="preview-error-message">{error.message}</span>
        </div>
        {#if error.details}
          <pre class="preview-error-details">{error.details}</pre>
        {/if}
        <button class="btn btn-secondary" onclick={() => fetchPdf(false)}>
          Retry
        </button>
      </div>
    {:else if pdfDataUrl}
      {#if notice}
        <div class="preview-notice" role="status" data-testid="pdf-notice">
          This resume has a saved LaTeX override. LaTeX is no longer compiled, so this PDF is
          generated from your resume content and does not include the override.
        </div>
      {/if}
      <iframe
        class="preview-iframe"
        src={pdfDataUrl}
        title="Resume PDF Preview"
      ></iframe>
    {/if}
  </div>
</div>

<style>
  .resume-preview {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .preview-notice {
    padding: 0.5rem 0.75rem;
    font-size: var(--text-sm);
    background: var(--color-warning-subtle, #fff7e0);
    color: var(--color-warning-text, #6b4e00);
    border-bottom: 1px solid var(--color-border);
  }

  .preview-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    flex-shrink: 0;
  }

  .cache-badge {
    margin-left: auto;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 600;
    font-family: monospace;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .cache-badge--hit {
    background: var(--color-success-subtle, #d1fae5);
    color: var(--color-success, #065f46);
  }

  .cache-badge--miss {
    background: var(--color-warning-subtle, #fef3c7);
    color: var(--color-warning, #92400e);
  }

  .preview-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .preview-iframe {
    width: 100%;
    height: 100%;
    border: none;
    flex: 1;
  }

  .preview-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 1rem;
    padding: 2rem;
    text-align: center;
    color: var(--text-muted);
  }

  .preview-loading-sub {
    font-size: var(--text-sm);
    color: var(--text-faint);
  }

  .preview-error {
    padding: 1.5rem;
  }

  .preview-error-header {
    display: flex;
    gap: 0.75rem;
    align-items: baseline;
    margin-bottom: 0.75rem;
  }

  .preview-error-code {
    padding: 0.2rem 0.5rem;
    background: var(--color-danger-subtle);
    color: var(--color-danger);
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    font-family: monospace;
  }

  .preview-error-message {
    font-weight: 500;
    color: var(--text-secondary);
  }

  .preview-error-details {
    background: var(--color-sidebar-bg);
    color: var(--color-sidebar-text-hover);
    padding: 1rem;
    border-radius: 6px;
    font-size: 0.8rem;
    overflow-x: auto;
    max-height: 300px;
    overflow-y: auto;
    margin-bottom: 1rem;
    white-space: pre-wrap;
  }

  .markdown-preview {
    padding: 2rem;
    overflow-y: auto;
    flex: 1;
    font-size: 0.95rem;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .markdown-preview :global(h1),
  .markdown-preview :global(h2),
  .markdown-preview :global(h3) {
    margin-top: 1.5em;
    margin-bottom: 0.5em;
    font-weight: 600;
    color: var(--text-primary);
  }

  .markdown-preview :global(ul),
  .markdown-preview :global(ol) {
    padding-left: 1.5rem;
    margin-bottom: 1em;
  }

  .markdown-preview :global(p) {
    margin-bottom: 0.75em;
  }

  .markdown-preview :global(code) {
    font-family: monospace;
    font-size: 0.85em;
    background: var(--color-surface-raised);
    padding: 0.1em 0.3em;
    border-radius: 3px;
  }

  .markdown-preview :global(pre) {
    background: var(--color-sidebar-bg);
    padding: 1rem;
    border-radius: 6px;
    overflow-x: auto;
    margin-bottom: 1em;
  }

  .markdown-preview :global(pre code) {
    background: none;
    padding: 0;
  }
</style>
