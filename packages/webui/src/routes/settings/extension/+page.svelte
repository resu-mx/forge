<script lang="ts">
  import { onMount } from 'svelte'
  import { PageHeader } from '$lib/components'
  import ExtensionUnavailable from '$lib/components/ExtensionUnavailable.svelte'
  import { extensionPagesAvailable } from '$lib/extension-gate'
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'

  let loading = $state(true)
  let saving = $state(false)

  let baseUrl = $state('')
  let devMode = $state(false)
  let enabledPlugins = $state('')
  let enableServerLogging = $state(true)

  async function loadConfig() {
    loading = true
    const result = await forge.extensionConfig.get()
    if (result.ok) {
      baseUrl = result.data.baseUrl
      devMode = result.data.devMode
      enabledPlugins = result.data.enabledPlugins.join(', ')
      enableServerLogging = result.data.enableServerLogging
    } else {
      addToast({ message: friendlyError(result.error), type: 'error' })
    }
    loading = false
  }

  async function saveConfig() {
    saving = true
    const plugins = enabledPlugins
      .split(',')
      .map(s => s.trim())
      .filter(s => s.length > 0)
    const result = await forge.extensionConfig.update({
      baseUrl,
      devMode,
      enabledPlugins: plugins,
      enableServerLogging,
    })
    if (result.ok) {
      addToast({ message: 'Config saved', type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to save config'), type: 'error' })
    }
    saving = false
  }

  // In wasm mode nothing may call /api/extension/*; the template branch alone would not stop this.
  onMount(() => {
    if (extensionPagesAvailable) loadConfig()
  })
</script>

<div class="settings-page">
  <PageHeader title="Extension Config" subtitle="Browser extension settings stored in Forge. The extension fetches this config on startup." />

  {#if !extensionPagesAvailable}
    <ExtensionUnavailable />
  {:else if loading}
    <div class="loading">Loading...</div>
  {:else}
    <form class="settings-form" onsubmit={(e) => { e.preventDefault(); saveConfig() }}>
      <div class="form-field">
        <label for="cfg-baseUrl">API Base URL</label>
        <input id="cfg-baseUrl" type="text" bind:value={baseUrl} />
        <span class="form-hint">The Forge API server URL the extension connects to.</span>
      </div>

      <div class="form-field">
        <label for="cfg-devMode">Dev Mode</label>
        <label class="toggle-label">
          <input id="cfg-devMode" type="checkbox" bind:checked={devMode} />
          <span>{devMode ? 'Enabled' : 'Disabled'}</span>
        </label>
        <span class="form-hint">Enables verbose console logging in the extension.</span>
      </div>

      <div class="form-field">
        <label for="cfg-plugins">Enabled Plugins</label>
        <input id="cfg-plugins" type="text" bind:value={enabledPlugins} />
        <span class="form-hint">Comma-separated list of enabled plugins (e.g. linkedin, workday).</span>
      </div>

      <div class="form-field">
        <label for="cfg-logging">Server Logging</label>
        <label class="toggle-label">
          <input id="cfg-logging" type="checkbox" bind:checked={enableServerLogging} />
          <span>{enableServerLogging ? 'Enabled' : 'Disabled'}</span>
        </label>
        <span class="form-hint">When enabled, extension errors are reported to the Forge server.</span>
      </div>

      <button type="submit" class="btn btn-primary" disabled={saving}>
        {saving ? 'Saving...' : 'Save Config'}
      </button>
    </form>
  {/if}
</div>

<style>
  .settings-page {
    max-width: 640px;
  }

  .settings-form {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .form-field > label:first-child {
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
  }

  .form-field input[type="text"] {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    background: var(--color-surface);
    transition: border-color 0.15s;
  }

  .form-field input[type="text"]:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-primary);
    cursor: pointer;
  }

  .toggle-label input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .form-hint {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .loading {
    text-align: center;
    padding: var(--space-8);
    color: var(--text-muted);
  }
</style>
