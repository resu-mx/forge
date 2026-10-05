<script lang="ts">
  import { onMount } from 'svelte'
  import { PageHeader, ConfirmDialog } from '$lib/components'
  import ExtensionUnavailable from '$lib/components/ExtensionUnavailable.svelte'
  import { extensionPagesAvailable } from '$lib/extension-gate'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import type { ExtensionLog } from '@forge/sdk'

  let loading = $state(true)
  let logs = $state<ExtensionLog[]>([])
  let expandedId = $state<string | null>(null)

  // Filters
  let filterCode = $state('')
  let filterLayer = $state('')

  // Pagination
  const PAGE_SIZE = 50
  let offset = $state(0)
  let hasMore = $state(false)

  // Clear confirmation
  let showClearConfirm = $state(false)

  async function loadLogs() {
    loading = true
    const opts: Record<string, unknown> = { limit: PAGE_SIZE + 1, offset }
    if (filterCode) opts.error_code = filterCode
    if (filterLayer) opts.layer = filterLayer

    const result = await forge.extensionLogs.list(opts as any)
    if (result.ok) {
      if (result.data.length > PAGE_SIZE) {
        hasMore = true
        logs = result.data.slice(0, PAGE_SIZE)
      } else {
        hasMore = false
        logs = result.data
      }
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
    loading = false
  }

  function toggleExpanded(id: string) {
    expandedId = expandedId === id ? null : id
  }

  function applyFilters() {
    offset = 0
    loadLogs()
  }

  function clearFilters() {
    filterCode = ''
    filterLayer = ''
    offset = 0
    loadLogs()
  }

  function nextPage() {
    offset += PAGE_SIZE
    loadLogs()
  }

  function prevPage() {
    offset = Math.max(0, offset - PAGE_SIZE)
    loadLogs()
  }

  async function clearAllLogs() {
    const result = await forge.extensionLogs.clear()
    if (result.ok) {
      addToast({ message: 'All logs cleared', type: 'success' })
      logs = []
      offset = 0
      hasMore = false
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to clear logs'), type: 'error' })
    }
    showClearConfirm = false
  }

  function formatTimestamp(ts: string): string {
    try {
      const d = new Date(ts + 'Z')
      return d.toLocaleString()
    } catch {
      return ts
    }
  }

  onMount(() => {
    if (extensionPagesAvailable) loadLogs()
  })
</script>

<div class="logs-page">
  <PageHeader title="Extension Logs" subtitle="Server-side error logs from the browser extension." />

  {#if !extensionPagesAvailable}
    <ExtensionUnavailable />
  {:else}
  <div class="toolbar">
    <div class="filters">
      <input
        type="text"
        placeholder="Filter by error code..."
        bind:value={filterCode}
        onkeydown={(e) => { if (e.key === 'Enter') applyFilters() }}
      />
      <select bind:value={filterLayer} onchange={applyFilters}>
        <option value="">All layers</option>
        <option value="plugin">plugin</option>
        <option value="content">content</option>
        <option value="background">background</option>
        <option value="popup">popup</option>
        <option value="sdk">sdk</option>
      </select>
      {#if filterCode || filterLayer}
        <button class="btn btn-ghost" onclick={clearFilters}>Clear filters</button>
      {/if}
    </div>
    <button
      class="btn btn-danger"
      onclick={() => showClearConfirm = true}
      disabled={logs.length === 0}
    >
      Clear All
    </button>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if logs.length === 0}
    <div class="empty">No extension logs recorded.</div>
  {:else}
    <div class="log-table-wrapper">
      <table class="log-table">
        <thead>
          <tr>
            <th>Timestamp</th>
            <th>Error Code</th>
            <th>Message</th>
            <th>Layer</th>
            <th>Plugin</th>
          </tr>
        </thead>
        <tbody>
          {#each logs as log}
            <tr
              class="log-row"
              class:expanded={expandedId === log.id}
              onclick={() => toggleExpanded(log.id)}
            >
              <td class="ts">{formatTimestamp(log.created_at)}</td>
              <td><code>{log.error_code}</code></td>
              <td class="msg">{log.message}</td>
              <td><span class="badge layer-{log.layer}">{log.layer}</span></td>
              <td>{log.plugin ?? '-'}</td>
            </tr>
            {#if expandedId === log.id && (log.url || log.context)}
              <tr class="detail-row">
                <td colspan="5">
                  <div class="detail-content">
                    {#if log.url}
                      <div class="detail-field">
                        <strong>URL:</strong> <span>{log.url}</span>
                      </div>
                    {/if}
                    {#if log.context}
                      <div class="detail-field">
                        <strong>Context:</strong>
                        <pre>{JSON.stringify(log.context, null, 2)}</pre>
                      </div>
                    {/if}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    <div class="pagination">
      <button class="btn btn-ghost" onclick={prevPage} disabled={offset === 0}>Previous</button>
      <span class="page-info">Showing {offset + 1}–{offset + logs.length}</span>
      <button class="btn btn-ghost" onclick={nextPage} disabled={!hasMore}>Next</button>
    </div>
  {/if}
  {/if}
</div>

{#if showClearConfirm}
  <ConfirmDialog
    title="Clear All Logs"
    message="This will permanently delete all extension error logs. This cannot be undone."
    confirmLabel="Clear All"
    onconfirm={clearAllLogs}
    oncancel={() => showClearConfirm = false}
  />
{/if}

<style>
  .logs-page {
    max-width: 1000px;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  .filters {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .filters input[type="text"] {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    background: var(--color-surface);
    width: 200px;
  }

  .filters input[type="text"]:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .filters select {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    background: var(--color-surface);
  }

  .log-table-wrapper {
    overflow-x: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
  }

  .log-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  .log-table th {
    text-align: left;
    padding: var(--space-3);
    background: var(--color-surface-sunken);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    border-bottom: 1px solid var(--color-border);
  }

  .log-table td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    vertical-align: top;
  }

  .log-row {
    cursor: pointer;
    transition: background 0.1s;
  }

  .log-row:hover {
    background: var(--color-surface-raised);
  }

  .log-row.expanded {
    background: var(--color-surface-raised);
  }

  .ts {
    white-space: nowrap;
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .msg {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    display: inline-block;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
  }

  .layer-sdk { background: var(--color-primary-subtle); color: var(--color-primary); }
  .layer-plugin { background: var(--color-success-subtle, #dcfce7); color: var(--color-success, #16a34a); }
  .layer-background { background: var(--color-warning-subtle, #fef9c3); color: var(--color-warning, #ca8a04); }
  .layer-content { background: var(--color-info-subtle, #dbeafe); color: var(--color-info, #2563eb); }
  .layer-popup { background: var(--color-surface-sunken); color: var(--text-secondary); }

  .detail-row td {
    padding: 0;
    border-bottom: 1px solid var(--color-border);
  }

  .detail-content {
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface-sunken);
  }

  .detail-field {
    margin-bottom: var(--space-2);
  }

  .detail-field:last-child {
    margin-bottom: 0;
  }

  .detail-field strong {
    color: var(--text-secondary);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .detail-field pre {
    margin-top: var(--space-1);
    padding: var(--space-2);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    overflow-x: auto;
    white-space: pre-wrap;
  }

  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-4);
  }

  .page-info {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .loading, .empty {
    text-align: center;
    padding: var(--space-8);
    color: var(--text-muted);
  }
</style>
