<!--
  SyncIndicator.svelte: one line under the account in the sidebar saying how cloud sync is doing.
  A conflict reopens the dialog; anything else goes to Settings, Storage, where the controls are.
  Renders nothing where cloud sync does not exist or nobody is signed in.
-->
<script lang="ts">
  import { goto } from '$app/navigation'
  import { onMount } from 'svelte'
  import { getSyncView, reopenConflict } from '$lib/stores/sync.svelte'
  import { statusLabel, statusTone } from '$lib/sync/copy'

  let view = $derived(getSyncView())
  // "Synced 2 min ago" needs a clock that moves.
  let now = $state(Date.now())
  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000)
    return () => clearInterval(timer)
  })

  let label = $derived(statusLabel(view, now))
  let tone = $derived(statusTone(view))

  function open() {
    if (view.conflict) reopenConflict()
    else void goto('/settings/storage')
  }
</script>

{#if view.available && view.signedIn && label}
  <button
    class="btn btn-ghost btn-xs sync-indicator"
    data-testid="sync-indicator"
    data-tone={tone}
    data-status={view.enabled ? view.status : 'off'}
    title={view.error ? view.error.message : label}
    onclick={open}
  >
    <span class="dot" aria-hidden="true"></span>
    <span class="label">{label}</span>
  </button>
{/if}

<style>
  /* Button look comes from the global .btn classes; this is only the dot-and-label layout. */
  .sync-indicator {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: var(--space-2);
    width: 100%;
    text-align: left;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    flex-shrink: 0;
    background: var(--text-faint);
  }

  [data-tone='ok'] .dot {
    background: var(--color-success, #2f9e5b);
  }

  [data-tone='busy'] .dot {
    background: var(--color-info, #3b82c4);
  }

  [data-tone='attention'] .dot {
    background: var(--color-warning, #d98c1f);
  }
</style>
