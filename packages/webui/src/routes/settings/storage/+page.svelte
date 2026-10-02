<script lang="ts">
  import { PageHeader, ConfirmDialog } from '$lib/components'
  import { forgeMode, runtime } from '$lib/sdk'
  import { getRuntimeStatus } from '$lib/stores/runtime.svelte'
  import { addToast } from '$lib/stores/toast.svelte'

  let status = $derived(getRuntimeStatus())
  let busy = $state(false)
  let pendingFile = $state<File | null>(null)
  let fileInput: HTMLInputElement | undefined = $state()

  async function exportDb() {
    if (!runtime) return
    busy = true
    try {
      const bytes = await runtime.exportDatabase()
      const blob = new Blob([bytes], { type: 'application/vnd.sqlite3' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `forge-${new Date().toISOString().slice(0, 10)}.db`
      a.click()
      URL.revokeObjectURL(url)
      addToast({ message: `Exported ${(bytes.byteLength / 1024).toFixed(0)} KB`, type: 'success' })
    } catch (e) {
      addToast({ message: `Export failed: ${e instanceof Error ? e.message : e}`, type: 'error' })
    } finally {
      busy = false
    }
  }

  function chooseFile(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0]
    if (file) pendingFile = file
  }

  async function confirmImport() {
    const file = pendingFile
    pendingFile = null
    if (!runtime || !file) return
    busy = true
    try {
      const info = await runtime.importDatabase(new Uint8Array(await file.arrayBuffer()))
      addToast({ message: `Imported ${(info.imported_bytes / 1024).toFixed(0)} KB. Reloading…`, type: 'success' })
      // Every open page holds data from the old database.
      setTimeout(() => location.reload(), 800)
    } catch (e) {
      addToast({ message: `Import failed: ${e instanceof Error ? e.message : e}`, type: 'error' })
    } finally {
      busy = false
      if (fileInput) fileInput.value = ''
    }
  }
</script>

<div class="settings-page">
  <PageHeader title="Storage" subtitle="Where your Forge data lives, and how to back it up." />

  {#if forgeMode !== 'wasm'}
    <div class="card" data-testid="storage-api-mode">
      <p>
        This Forge talks to an API server, which owns your data. Back it up on the server
        (<code>data/forge.db</code>). The in-browser database is not in use.
      </p>
    </div>
  {:else}
    <div class="card" data-testid="storage-status">
      <h3>Status</h3>
      {#if status?.state === 'ready'}
        <dl>
          <dt>Database</dt><dd>Open in this tab (browser storage, OPFS)</dd>
          <dt>Migrations applied</dt><dd>{status.info.migrations ?? 'unknown'}</dd>
          <dt>Protected from eviction</dt>
          <dd>{status.persisted === null ? 'Unknown' : status.persisted ? 'Yes' : 'No — the browser may clear it under storage pressure'}</dd>
          <dt>Runtime</dt><dd>forge-wasm {status.info.version ?? ''}</dd>
        </dl>
      {:else if status?.state === 'waiting'}
        <p>Forge is open in another tab. This tab will take over when that tab closes.</p>
      {:else if status?.state === 'error'}
        <p class="error">{status.message}</p>
      {:else}
        <p>Starting…</p>
      {/if}
    </div>

    <div class="card">
      <h3>Backup</h3>
      <p>
        Download the whole database as a SQLite file. Your data lives only in this browser's
        storage, so do this before clearing site data or switching browsers.
      </p>
      <button class="btn btn-primary" onclick={exportDb} disabled={busy || status?.state !== 'ready'} data-testid="export-db">
        Export database
      </button>
    </div>

    <div class="card">
      <h3>Restore or move in</h3>
      <p>
        Replace everything in this browser with a SQLite file, for example a backup from here,
        or a <code>forge.db</code> from the server. A file from the server must be checkpointed
        first (<code>sqlite3 forge.db "PRAGMA wal_checkpoint(TRUNCATE)"</code>) so no
        <code>-wal</code> file is left behind.
      </p>
      <input
        type="file"
        accept=".db,.sqlite,.sqlite3,application/vnd.sqlite3"
        bind:this={fileInput}
        onchange={chooseFile}
        disabled={busy || status?.state !== 'ready'}
        data-testid="import-file"
      />
    </div>
  {/if}
</div>

<ConfirmDialog
  open={pendingFile !== null}
  title="Replace all data?"
  message={`Everything in this browser will be replaced by "${pendingFile?.name ?? ''}". Export a backup first if you might want the current data back. A file that is not a valid Forge database is rejected and nothing changes.`}
  confirmLabel="Replace data"
  onconfirm={confirmImport}
  oncancel={() => { pendingFile = null; if (fileInput) fileInput.value = '' }}
/>

<style>
  .settings-page {
    max-width: 720px;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  h3 {
    margin: 0;
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
  }
  p { margin: 0; color: var(--text-secondary); font-size: var(--text-sm); }
  dl { margin: 0; display: grid; grid-template-columns: max-content 1fr; gap: var(--space-2) var(--space-5); font-size: var(--text-sm); }
  dt { color: var(--text-secondary); }
  dd { margin: 0; }
  .error { color: var(--color-danger-text, #8a1c1c); }
  code { font-size: 0.9em; }
</style>
