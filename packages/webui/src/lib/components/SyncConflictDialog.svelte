<!--
  SyncConflictDialog.svelte: asks which copy to keep when this browser and the cloud disagree
  (or the cloud copy was deleted). Mounted once in the app layout; it opens itself when the sync
  engine reports a conflict. Nothing is replaced until a button is pressed, and "Export this
  browser's data first" is always on offer.
-->
<script lang="ts">
  import Modal from './Modal.svelte'
  import {
    dismissConflict,
    exportBeforeResolving,
    getSyncView,
    isConflictDismissed,
    resolveSyncConflict,
  } from '$lib/stores/sync.svelte'
  import { conflictCopy } from '$lib/sync/copy'

  let view = $derived(getSyncView())
  let copy = $derived(view.conflict ? conflictCopy(view.conflict) : null)
  let open = $derived(view.conflict !== null && !isConflictDismissed())
  let exporting = $state(false)

  async function exportFirst() {
    exporting = true
    try {
      await exportBeforeResolving()
    } finally {
      exporting = false
    }
  }
</script>

{#if copy}
  <Modal {open} onClose={dismissConflict} size="md" title={copy.title}>
    {#snippet body()}
      <div class="sync-conflict" data-testid="sync-conflict">
        <p class="sync-conflict-body">{copy.body}</p>
        <ul class="sync-choices">
          {#each copy.choices as c (c.choice)}
            <li class="sync-choice">
              <button
                class="btn"
                class:btn-primary={c.tone === 'primary'}
                class:btn-secondary={c.tone === 'secondary'}
                class:btn-ghost={c.tone === 'ghost'}
                data-testid={`sync-choice-${c.choice}`}
                disabled={view.busy}
                onclick={() => resolveSyncConflict(c.choice)}
              >
                {c.label}
              </button>
              <span class="sync-choice-desc">{c.description}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/snippet}
    {#snippet footer()}
      <button class="btn btn-ghost" data-testid="sync-export-first" disabled={exporting} onclick={exportFirst}>
        Export this browser's data first
      </button>
      <button class="btn btn-ghost" data-testid="sync-decide-later" onclick={dismissConflict}>Decide later</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .sync-conflict {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .sync-conflict-body {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-base);
    line-height: var(--leading-normal);
  }

  .sync-choices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .sync-choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
  }

  .sync-choice-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
</style>
