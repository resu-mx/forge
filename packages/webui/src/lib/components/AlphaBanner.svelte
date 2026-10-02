<script lang="ts">
  import { onMount } from 'svelte'
  import { forgeMode } from '$lib/sdk'

  const KEY = 'forge:alpha-banner-dismissed'

  // Only the browser-first app needs this: its data lives in this browser alone.
  // Start hidden so a returning user who dismissed it never sees a flash.
  let visible = $state(false)

  onMount(() => {
    if (forgeMode !== 'wasm') return
    try {
      visible = localStorage.getItem(KEY) !== '1'
    } catch {
      visible = true  // storage blocked: show it, dismissal just won't persist
    }
  })

  function dismiss() {
    visible = false
    try {
      localStorage.setItem(KEY, '1')
    } catch {
      // Not persisted; it will show again next load.
    }
  }
</script>

{#if visible}
  <div class="banner" role="note" data-testid="alpha-banner">
    <span>
      <strong>Alpha.</strong>
      Works in Chrome only. Your data lives only in this browser, so export it regularly
      (<a href="/settings/storage">Settings → Storage</a>).
    </span>
    <button class="dismiss" onclick={dismiss} aria-label="Dismiss">×</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    font-size: var(--text-sm);
    border-bottom: 1px solid var(--color-border);
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
