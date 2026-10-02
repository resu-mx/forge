<script lang="ts">
  import { getRuntimeStatus } from '$lib/stores/runtime.svelte'

  let status = $derived(getRuntimeStatus())
  let dismissedPersist = $state(false)

  // Shown only for states the user needs to know about; a healthy runtime is silent.
  let showPersistNotice = $derived(status?.state === 'ready' && status.persisted === false && !dismissedPersist)
</script>

{#if status?.state === 'waiting'}
  <div class="banner banner-warning" role="status" data-testid="storage-banner">
    <strong>Forge is open in another tab.</strong>
    Your data lives in that tab. This tab will take over when you close it, or switch to it now.
  </div>
{:else if status?.state === 'error'}
  <div class="banner banner-error" role="alert" data-testid="storage-banner">
    <strong>The in-browser database could not be opened.</strong>
    {status.message}
  </div>
{:else if showPersistNotice}
  <div class="banner banner-info" role="status" data-testid="storage-banner">
    <span>
      <strong>Your browser may clear this data under storage pressure.</strong>
      Export a backup from <a href="/settings/storage">Settings → Storage</a>.
    </span>
    <button class="dismiss" onclick={() => (dismissedPersist = true)} aria-label="Dismiss">×</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    font-size: var(--text-sm);
    border-bottom: 1px solid var(--color-border);
  }
  .banner-warning {
    background: var(--color-warning-subtle, #fff7e0);
    color: var(--color-warning-text, #6b4e00);
  }
  .banner-error {
    background: var(--color-danger-subtle, #fdecec);
    color: var(--color-danger-text, #8a1c1c);
  }
  .banner-info {
    background: var(--color-primary-subtle, #eef3ff);
    color: var(--text-primary);
  }
  .dismiss {
    background: none;
    border: none;
    cursor: pointer;
    font-size: var(--text-lg);
    line-height: 1;
    color: inherit;
  }
</style>
