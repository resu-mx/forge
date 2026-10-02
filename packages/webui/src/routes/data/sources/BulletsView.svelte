<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { onForgeChanged } from '$lib/forge-changed'
  import { StatusBadge, LoadingSpinner, EmptyState, PageHeader, ListSearchInput } from '$lib/components'
  import BulletDetailModal from '$lib/components/BulletDetailModal.svelte'
  import ViewToggle from '$lib/components/ViewToggle.svelte'
  import GenericKanban from '$lib/components/kanban/GenericKanban.svelte'
  import BulletKanbanCard from '$lib/components/kanban/BulletKanbanCard.svelte'
  import PerspectiveKanbanCard from '$lib/components/kanban/PerspectiveKanbanCard.svelte'
  import BulletFilterBar from '$lib/components/filters/BulletFilterBar.svelte'
  import PerspectiveFilterBar from '$lib/components/filters/PerspectiveFilterBar.svelte'
  import { getViewMode, setViewMode } from '$lib/stores/viewMode.svelte'
  import type { Bullet, Perspective } from '@forge/sdk'

  type ContentType = 'bullet' | 'perspective'

  let contentType = $state<ContentType>('bullet')
  let items = $state<any[]>([])
  let loading = $state(true)
  let searchQuery = $state('')
  let statusFilter = $state('all')

  // View mode: list or board
  let viewMode = $state<'list' | 'board'>(getViewMode('bullets'))

  function handleViewChange(mode: 'list' | 'board') {
    viewMode = mode
    setViewMode(contentType === 'bullet' ? 'bullets' : 'perspectives', mode)
  }

  // Column definitions for kanban boards
  const BULLET_COLUMNS = [
    { key: 'draft', label: 'Draft', statuses: ['draft'], accent: '#a5b4fc' },
    { key: 'in_review', label: 'In Review', statuses: ['in_review'], accent: '#fbbf24' },
    { key: 'approved', label: 'Approved', statuses: ['approved'], accent: '#22c55e' },
    { key: 'rejected', label: 'Rejected', statuses: ['rejected'], accent: '#ef4444' },
    { key: 'archived', label: 'Archived', statuses: ['archived'], accent: '#d1d5db' },
  ]

  const PERSPECTIVE_COLUMNS = [
    { key: 'draft', label: 'Draft', statuses: ['draft'], accent: '#a5b4fc' },
    { key: 'in_review', label: 'In Review', statuses: ['in_review'], accent: '#fbbf24' },
    { key: 'approved', label: 'Approved', statuses: ['approved'], accent: '#22c55e' },
    { key: 'rejected', label: 'Rejected', statuses: ['rejected'], accent: '#ef4444' },
    { key: 'archived', label: 'Archived', statuses: ['archived'], accent: '#d1d5db' },
  ]

  // Board filter state
  let bulletBoardFilters = $state<{ source?: string; domain?: string; search?: string }>({})
  let perspectiveBoardFilters = $state<{ archetype?: string; domain?: string; framing?: string; search?: string }>({})

  let boardFilteredItems = $derived.by(() => {
    if (contentType === 'bullet') {
      let result = items
      if (bulletBoardFilters.domain) result = result.filter((b: any) => b.domain === bulletBoardFilters.domain)
      if (bulletBoardFilters.search) {
        const q = bulletBoardFilters.search.toLowerCase()
        result = result.filter((b: any) => b.content.toLowerCase().includes(q))
      }
      return result
    } else {
      let result = items
      if (perspectiveBoardFilters.archetype) result = result.filter((p: any) => p.target_archetype === perspectiveBoardFilters.archetype)
      if (perspectiveBoardFilters.domain) result = result.filter((p: any) => p.domain === perspectiveBoardFilters.domain)
      if (perspectiveBoardFilters.framing) result = result.filter((p: any) => p.framing === perspectiveBoardFilters.framing)
      if (perspectiveBoardFilters.search) {
        const q = perspectiveBoardFilters.search.toLowerCase()
        result = result.filter((p: any) => p.content.toLowerCase().includes(q))
      }
      return result
    }
  })

  async function handleBoardDrop(itemId: string, newStatus: string) {
    if (contentType === 'bullet') {
      const result = await forge.bullets.update(itemId, { status: newStatus } as any)
      if (!result.ok) {
        addToast({ type: 'error', message: friendlyError(result.error, 'Status update failed') })
        throw new Error('Status update failed')
      }
      items = items.map(i => i.id === itemId ? result.data : i)
      addToast({ type: 'success', message: `Bullet moved to ${newStatus.replace('_', ' ')}` })
    } else {
      const result = await forge.perspectives.update(itemId, { status: newStatus } as any)
      if (!result.ok) {
        addToast({ type: 'error', message: friendlyError(result.error, 'Status update failed') })
        throw new Error('Status update failed')
      }
      items = items.map(i => i.id === itemId ? result.data : i)
      addToast({ type: 'success', message: `Perspective moved to ${newStatus.replace('_', ' ')}` })
    }
  }

  // Bullet detail modal
  let detailBulletId = $state<string | null>(null)

  // Reject modal
  let rejectModal = $state<{
    open: boolean
    type: ContentType
    id: string
    reason: string
  }>({ open: false, type: 'bullet', id: '', reason: '' })

  let filteredItems = $derived.by(() => {
    let result = items
    if (statusFilter !== 'all') {
      result = result.filter((i: any) => i.status === statusFilter)
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase()
      result = result.filter((i: any) => i.content.toLowerCase().includes(q))
    }
    return result
  })

  $effect(() => {
    loadItems()
  })

  // An agent (or another part of the page) changed data: refresh in place, keeping the list on screen.
  $effect(() => onForgeChanged(() => loadItems(true)))

  async function loadItems(quiet = false) {
    if (!quiet) {
      loading = true
      items = []
    }

    if (contentType === 'bullet') {
      const [bulletRes, perspRes] = await Promise.all([
        forge.bullets.list({ limit: 500 }),
        forge.perspectives.list({ limit: 2000 }),
      ])
      if (bulletRes.ok) items = bulletRes.data
      else addToast({ message: friendlyError(bulletRes.error, 'Failed to load bullets'), type: 'error' })

      // Group perspectives by bullet_id for the accordion
      if (perspRes.ok) {
        const map: Record<string, Perspective[]> = {}
        for (const p of perspRes.data) {
          if (!map[p.bullet_id]) map[p.bullet_id] = []
          map[p.bullet_id].push(p)
        }
        perspectivesByBullet = map
      }
    } else if (contentType === 'perspective') {
      const result = await forge.perspectives.list({ limit: 500 })
      if (result.ok) items = result.data
      else addToast({ message: friendlyError(result.error, 'Failed to load perspectives'), type: 'error' })
    }

    loading = false
  }

  function switchType(type: ContentType) {
    contentType = type
    statusFilter = 'all'
    searchQuery = ''
  }

  async function approveItem(id: string) {
    if (contentType === 'bullet') {
      const res = await forge.bullets.approve(id)
      if (res.ok) {
        items = items.map(i => i.id === id ? res.data : i)
        addToast({ message: 'Bullet approved', type: 'success' })
      } else {
        addToast({ message: `Approve failed: ${res.error.message}`, type: 'error' })
      }
    } else if (contentType === 'perspective') {
      const res = await forge.perspectives.approve(id)
      if (res.ok) {
        items = items.map(i => i.id === id ? res.data : i)
        addToast({ message: 'Perspective approved', type: 'success' })
      } else {
        addToast({ message: `Approve failed: ${res.error.message}`, type: 'error' })
      }
    }
  }

  function openReject(id: string) {
    rejectModal = { open: true, type: contentType, id, reason: '' }
  }

  async function submitReject() {
    if (!rejectModal.reason.trim()) {
      addToast({ message: 'Please provide a rejection reason.', type: 'error' })
      return
    }

    const { type, id, reason } = rejectModal

    if (type === 'bullet') {
      const res = await forge.bullets.reject(id, { rejection_reason: reason })
      if (res.ok) {
        items = items.map(i => i.id === id ? res.data : i)
        addToast({ message: 'Bullet rejected', type: 'success' })
      } else {
        addToast({ message: `Reject failed: ${res.error.message}`, type: 'error' })
      }
    } else if (type === 'perspective') {
      const res = await forge.perspectives.reject(id, { rejection_reason: reason })
      if (res.ok) {
        items = items.map(i => i.id === id ? res.data : i)
        addToast({ message: 'Perspective rejected', type: 'success' })
      } else {
        addToast({ message: `Reject failed: ${res.error.message}`, type: 'error' })
      }
    }

    rejectModal = { open: false, type: 'bullet', id: '', reason: '' }
  }

  async function reopenItem(id: string) {
    if (contentType === 'bullet') {
      const res = await forge.bullets.reopen(id)
      if (res.ok) {
        items = items.map(i => i.id === id ? res.data : i)
        addToast({ message: 'Bullet reopened', type: 'success' })
      } else {
        addToast({ message: `Reopen failed: ${res.error.message}`, type: 'error' })
      }
    } else if (contentType === 'perspective') {
      const res = await forge.perspectives.reopen(id)
      if (res.ok) {
        items = items.map(i => i.id === id ? res.data : i)
        addToast({ message: 'Perspective reopened', type: 'success' })
      } else {
        addToast({ message: `Reopen failed: ${res.error.message}`, type: 'error' })
      }
    }
  }

  // ── Perspective accordion (bullets tab, list view only) ──────

  /** Map of bullet_id → perspectives[] for the accordion. */
  let perspectivesByBullet = $state<Record<string, Perspective[]>>({})
  /** Set of bullet IDs whose accordion is currently expanded. */
  let expandedBullets = $state<Set<string>>(new Set())

  function toggleAccordion(bulletId: string, e: MouseEvent) {
    e.stopPropagation()
    const next = new Set(expandedBullets)
    if (next.has(bulletId)) next.delete(bulletId)
    else next.add(bulletId)
    expandedBullets = next
  }

  function perspectiveCount(bulletId: string): number {
    return perspectivesByBullet[bulletId]?.length ?? 0
  }

  // ── Manual bullet creation ────────────────────────────────────

  let showCreateForm = $state(false)
  let createContent = $state('')
  let createDomain = $state('')
  let createSourceId = $state<string | null>(null)
  let creating = $state(false)
  let allSources = $state<Array<{ id: string; title: string; source_type: string }>>([])

  async function openCreateForm() {
    showCreateForm = true
    createContent = ''
    createDomain = ''
    createSourceId = null
    // Load sources for the dropdown (if not already loaded)
    if (allSources.length === 0) {
      const res = await forge.sources.list({ limit: 500 })
      if (res.ok) {
        allSources = res.data.map(s => ({ id: s.id, title: s.title, source_type: s.source_type }))
      }
    }
  }

  async function createBullet() {
    if (!createContent.trim()) {
      addToast({ message: 'Content is required', type: 'error' })
      return
    }
    creating = true
    const res = await forge.bullets.create({
      content: createContent.trim(),
      domain: createDomain.trim() || undefined,
      source_ids: createSourceId ? [{ id: createSourceId, is_primary: true }] : undefined,
    } as any)
    if (res.ok) {
      items = [res.data, ...items]
      showCreateForm = false
      addToast({ message: 'Bullet created (draft)', type: 'success' })
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to create bullet'), type: 'error' })
    }
    creating = false
  }

  async function deleteBullet(id: string) {
    const res = await forge.bullets.delete(id)
    if (res.ok) {
      items = items.filter(i => i.id !== id)
      addToast({ message: 'Bullet deleted', type: 'success' })
    } else {
      addToast({ message: friendlyError(res.error, 'Failed to delete'), type: 'error' })
    }
  }

  function truncate(text: string, max: number = 200): string {
    if (text.length <= max) return text
    return text.slice(0, max) + '...'
  }
</script>

<div class="bullets-page">
  <PageHeader title="Content Atoms" subtitle="Unified view of bullets and perspectives">
    {#snippet actions()}
      {#if contentType === 'bullet'}
        <button class="btn btn-primary" onclick={openCreateForm}>+ New Bullet</button>
      {/if}
      <ViewToggle mode={viewMode} onchange={handleViewChange} />
    {/snippet}
  </PageHeader>

  <!-- Inline create form -->
  {#if showCreateForm}
    <div class="create-form">
      <h4>New Bullet (Manual)</h4>
      <div class="create-form-field">
        <label for="create-content">Content <span style="color: var(--color-danger);">*</span></label>
        <textarea
          id="create-content"
          bind:value={createContent}
          rows="3"
          placeholder="Describe an accomplishment, responsibility, or context..."
          class="create-textarea"
        ></textarea>
      </div>
      <div class="create-form-field">
        <label for="create-source">Source (optional)</label>
        <select id="create-source" bind:value={createSourceId} class="create-input">
          <option value={null}>-- No source --</option>
          {#each allSources as src (src.id)}
            <option value={src.id}>[{src.source_type}] {src.title}</option>
          {/each}
        </select>
      </div>
      <div class="create-form-field">
        <label for="create-domain">Domain (optional)</label>
        <input
          id="create-domain"
          type="text"
          bind:value={createDomain}
          placeholder="e.g., security, infrastructure, ai_ml"
          class="create-input"
        />
      </div>
      <div class="create-actions">
        <button class="btn btn-primary btn-sm" onclick={createBullet} disabled={creating}>
          {creating ? 'Creating...' : 'Create Draft'}
        </button>
        <button class="btn btn-ghost btn-sm" onclick={() => showCreateForm = false}>Cancel</button>
      </div>
    </div>
  {/if}

  <!-- Controls -->
  <div class="controls">
    <div class="type-tabs">
      {#each [
        { value: 'bullet', label: 'Bullets' },
        { value: 'perspective', label: 'Perspectives' },
      ] as tab}
        <button
          class="type-tab"
          class:active={contentType === tab.value}
          onclick={() => switchType(tab.value as ContentType)}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <ListSearchInput bind:value={searchQuery} placeholder="Search content..." />

    <select class="status-select" bind:value={statusFilter}>
      <option value="all">All statuses</option>
      <option value="in_review">In Review</option>
      <option value="approved">Approved</option>
      <option value="rejected">Rejected</option>
      <option value="draft">Draft</option>
      <option value="archived">Archived</option>
    </select>
  </div>

  {#if viewMode === 'board'}
    <!-- Board view -->
    {#if contentType === 'bullet'}
      <GenericKanban
        columns={BULLET_COLUMNS}
        items={boardFilteredItems}
        onDrop={handleBoardDrop}
        {loading}
        emptyMessage="No bullets yet. Create sources and derive bullets to populate this board."
        defaultCollapsed="archived"
        sortItems={(a, b) => a.content.localeCompare(b.content)}
      >
        {#snippet filterBar()}
          <BulletFilterBar bind:filters={bulletBoardFilters} onchange={() => {}} />
        {/snippet}

        {#snippet cardContent(bullet)}
          <BulletKanbanCard {bullet} onclick={() => detailBulletId = bullet.id} />
        {/snippet}
      </GenericKanban>
    {:else}
      <GenericKanban
        columns={PERSPECTIVE_COLUMNS}
        items={boardFilteredItems}
        onDrop={handleBoardDrop}
        {loading}
        emptyMessage="No perspectives yet. Derive perspectives from approved bullets."
        defaultCollapsed="archived"
        sortItems={(a, b) => a.content.localeCompare(b.content)}
      >
        {#snippet filterBar()}
          <PerspectiveFilterBar bind:filters={perspectiveBoardFilters} onchange={() => {}} />
        {/snippet}

        {#snippet cardContent(perspective)}
          <PerspectiveKanbanCard {perspective} onclick={() => detailBulletId = perspective.bullet_id} />
        {/snippet}
      </GenericKanban>
    {/if}
  {:else}
  <!-- Item list (list view) -->
  {#if loading}
    <div class="loading-container">
      <LoadingSpinner size="lg" message="Loading {contentType}s..." />
    </div>
  {:else if filteredItems.length === 0}
    <EmptyState
      title="No {contentType}s found"
      description={searchQuery ? 'Try adjusting your search.' : 'No items match the current filters.'}
    />
  {:else}
    <div class="item-list">
      {#each filteredItems as item (item.id)}
        <div class="item-card" style="cursor: pointer;" onclick={() => detailBulletId = contentType === 'perspective' ? item.bullet_id : item.id}>
          <div class="item-header">
            <p class="item-content">{truncate(item.content, 200)}</p>
            <StatusBadge status={item.status} />
          </div>

          <!-- Bullet-specific metadata -->
          {#if contentType === 'bullet'}
            {#if item.sources?.length > 0}
              <div class="meta-row">
                <span class="meta-label">Sources:</span>
                {#each item.sources as src}
                  <span class="source-tag" class:primary={src.is_primary}>
                    {src.title}
                  </span>
                {/each}
              </div>
            {/if}
            {#if item.technologies?.length > 0}
              <div class="tech-tags">
                {#each item.technologies as tech}
                  <span class="tech-tag">{tech}</span>
                {/each}
              </div>
            {/if}

            <!-- Perspective accordion -->
            {#if perspectiveCount(item.id) > 0}
              <button
                class="accordion-toggle"
                onclick={(e) => toggleAccordion(item.id, e)}
              >
                <span class="accordion-chevron" class:expanded={expandedBullets.has(item.id)}>&#9656;</span>
                <span class="accordion-label">
                  {perspectiveCount(item.id)} perspective{perspectiveCount(item.id) !== 1 ? 's' : ''}
                </span>
              </button>
              {#if expandedBullets.has(item.id)}
                <div class="accordion-content">
                  {#each perspectivesByBullet[item.id] ?? [] as persp (persp.id)}
                    <div class="accordion-perspective">
                      <p class="persp-content">{truncate(persp.content, 160)}</p>
                      <div class="persp-meta">
                        {#if persp.target_archetype}
                          <span class="meta-tag archetype">{persp.target_archetype}</span>
                        {/if}
                        {#if persp.domain}
                          <span class="meta-tag domain">{persp.domain}</span>
                        {/if}
                        <span class="meta-tag framing">{persp.framing}</span>
                        <StatusBadge status={persp.status} />
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            {:else}
              <span class="accordion-empty">No perspectives derived yet</span>
            {/if}
          {/if}

          <!-- Perspective-specific metadata -->
          {#if contentType === 'perspective'}
            <div class="perspective-meta">
              {#if item.target_archetype}
                <span class="meta-tag archetype">{item.target_archetype}</span>
              {/if}
              {#if item.domain}
                <span class="meta-tag domain">{item.domain}</span>
              {/if}
              <span class="meta-tag framing">{item.framing}</span>
            </div>
          {/if}

          {#if item.rejection_reason}
            <p class="rejection-reason">Reason: {item.rejection_reason}</p>
          {/if}

          <!-- Status actions live on the kanban board + BulletDetailModal.
               Delete lives in the modal. Cards are for scan + click to open. -->
        </div>
      {/each}
    </div>
  {/if}
  {/if}
</div>

<!-- Reject Modal -->
{#if rejectModal.open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => rejectModal.open = false} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Reject Item">
      <div class="modal-header">
        <h3>Reject {rejectModal.type}</h3>
        <button class="btn btn-ghost" onclick={() => rejectModal.open = false}>Close</button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label for="reject-reason">Reason <span class="required">*</span></label>
          <textarea
            id="reject-reason"
            bind:value={rejectModal.reason}
            rows="4"
            placeholder="Why is this being rejected?"
          ></textarea>
        </div>
        <div class="modal-actions">
          <button class="btn btn-ghost" onclick={() => rejectModal.open = false}>Cancel</button>
          <button class="btn btn-danger" onclick={submitReject}>Reject</button>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if detailBulletId}
  <BulletDetailModal
    bulletId={detailBulletId}
    onclose={() => detailBulletId = null}
    onupdate={() => loadItems()}
  />
{/if}

<style>
  .bullets-page {
    max-width: 1000px;
  }

  .create-form {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    margin-bottom: 1.25rem;
    background: var(--color-surface);
  }

  .create-form h4 {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin: 0 0 0.75rem;
  }

  .create-form-field {
    margin-bottom: 0.75rem;
  }

  .create-form-field label {
    display: block;
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    color: var(--text-secondary);
    margin-bottom: 0.3rem;
  }

  .create-textarea,
  .create-input {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    color: var(--text-primary);
    background: var(--color-surface);
    font-family: inherit;
  }

  .create-textarea:focus,
  .create-input:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .create-textarea {
    resize: vertical;
    min-height: 80px;
    line-height: 1.5;
  }

  .create-actions {
    display: flex;
    gap: 0.5rem;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .type-tabs {
    display: flex;
    background: var(--color-ghost);
    border-radius: 6px;
    padding: 3px;
  }

  .type-tab {
    padding: 0.4rem 0.8rem;
    background: none;
    border: none;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .type-tab.active {
    background: var(--color-surface);
    color: var(--color-primary);
    box-shadow: var(--shadow-sm);
  }

  .status-select {
    padding: 0.45rem 0.75rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 6px;
    font-size: 0.825rem;
    color: var(--text-primary);
    background: var(--color-surface);
  }

  .status-select:focus {
    outline: none;
    border-color: var(--color-border-focus);
  }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 4rem 0;
  }

  .item-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .item-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1rem 1.25rem;
  }

  .item-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .item-content {
    font-size: 0.875rem;
    color: var(--text-secondary);
    line-height: 1.5;
    flex: 1;
  }

  .meta-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
    margin-bottom: 0.35rem;
  }

  .meta-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .source-tag {
    display: inline-block;
    padding: 0.12em 0.45em;
    background: var(--color-tag-neutral-bg);
    color: var(--color-tag-neutral-text);
    border-radius: 3px;
    font-size: 0.72rem;
    font-weight: 500;
  }

  .source-tag.primary {
    background: var(--color-info-subtle);
    color: var(--color-info-text);
  }

  .tech-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.35rem;
  }

  .tech-tag {
    display: inline-block;
    padding: 0.12em 0.45em;
    background: var(--color-tag-bg);
    color: var(--color-tag-text);
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 500;
  }

  .perspective-meta {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
    margin-bottom: 0.35rem;
  }

  .meta-tag {
    display: inline-block;
    padding: 0.12em 0.45em;
    border-radius: 3px;
    font-size: 0.72rem;
    font-weight: 500;
  }

  .meta-tag.archetype { background: var(--color-primary-subtle); color: var(--color-tag-text); }
  .meta-tag.domain { background: var(--color-success-subtle); color: var(--color-success-text); }
  .meta-tag.framing { background: var(--color-warning-subtle); color: var(--color-warning-text); }

  .rejection-reason {
    font-size: 0.78rem;
    color: var(--color-danger);
    font-style: italic;
    margin-bottom: 0.35rem;
  }

  /* Perspective accordion */
  .accordion-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: none;
    border: none;
    cursor: pointer;
    font-size: var(--text-sm);
    color: var(--color-primary);
    font-weight: var(--font-medium);
    font-family: inherit;
    padding: 0.25rem 0;
    margin-top: 0.35rem;
  }

  .accordion-toggle:hover {
    text-decoration: underline;
  }

  .accordion-chevron {
    display: inline-block;
    font-size: var(--text-xs);
    transition: transform 0.15s ease;
  }

  .accordion-chevron.expanded {
    transform: rotate(90deg);
  }

  .accordion-empty {
    font-size: var(--text-xs);
    color: var(--text-faint);
    font-style: italic;
    display: block;
    margin-top: 0.35rem;
  }

  .accordion-content {
    margin-top: 0.5rem;
    padding-left: 0.75rem;
    border-left: 2px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .accordion-perspective {
    padding: 0.4rem 0.6rem;
    background: var(--color-surface-sunken);
    border-radius: var(--radius-sm);
  }

  .persp-content {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: 1.4;
    margin: 0 0 0.25rem;
  }

  .persp-meta {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
    align-items: center;
  }

  /* Modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 10000;
    background: var(--color-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: var(--color-surface);
    border-radius: 8px;
    width: 90%;
    max-width: 480px;
    box-shadow: var(--shadow-lg);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--color-border);
  }

  .modal-header h3 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .modal-body {
    padding: 1.25rem;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 0.35rem;
  }

  .required { color: var(--color-danger); }

  .form-group textarea,
  .form-group select {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 6px;
    font-size: 0.875rem;
    color: var(--text-primary);
    background: var(--color-surface);
    font-family: inherit;
  }

  .form-group textarea:focus,
  .form-group select:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .form-group textarea {
    resize: vertical;
    min-height: 80px;
    line-height: 1.5;
  }
</style>
