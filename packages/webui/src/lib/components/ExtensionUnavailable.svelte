<script lang="ts">
  import EmptyState from './EmptyState.svelte'
  import { EXTENSION_UNAVAILABLE_MESSAGE, EXTENSION_UNAVAILABLE_TITLE } from '$lib/extension-gate'
</script>

<div data-testid="extension-unavailable">
  <EmptyState title={EXTENSION_UNAVAILABLE_TITLE} description={EXTENSION_UNAVAILABLE_MESSAGE} />
</div>
