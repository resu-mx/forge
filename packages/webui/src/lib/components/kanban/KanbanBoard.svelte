<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { LoadingSpinner } from '$lib/components'
  import KanbanColumn from './KanbanColumn.svelte'
  import OrgPickerModal from './OrgPickerModal.svelte'
  import OrgDetailModal from './OrgDetailModal.svelte'
  import { hqLocationLabel } from '$lib/org-locations'
  import type { Organization } from '@forge/sdk'

  type OrgStatus = 'backlog' | 'researching' | 'exciting' | 'interested' | 'acceptable' | 'excluded'

  const COLUMNS: Array<{
    key: string
    label: string
    statuses: OrgStatus[]
    accent: string
  }> = [
    { key: 'backlog', label: 'Backlog', statuses: ['backlog'], accent: '#a5b4fc' },
    { key: 'researching', label: 'Researching', statuses: ['researching'], accent: '#fbbf24' },
    { key: 'targeting', label: 'Targeting', statuses: ['exciting', 'interested', 'acceptable'], accent: '#22c55e' },
    { key: 'excluded', label: 'Excluded', statuses: ['excluded'], accent: '#d1d5db' },
  ]

  // Default status when dropping into a column
  const DROP_STATUS: Record<string, OrgStatus> = {
    backlog: 'backlog',
    researching: 'researching',
    targeting: 'interested', // default interest level
    excluded: 'excluded',
  }

  let organizations = $state<Organization[]>([])
  let loading = $state(true)
  let excludedExpanded = $state(false)
  let showPicker = $state(false)
  let detailOrgId = $state<string | null>(null)

  // Enrichment data: alias counts and HQ locations per org
  let aliasCountMap = $state<Map<string, number>>(new Map())
  let hqLocationMap = $state<Map<string, string>>(new Map())

  let detailOrg = $derived(organizations.find(o => o.id === detailOrgId) ?? null)

  let columnData = $derived(COLUMNS.map(col => ({
    ...col,
    items: organizations
      .filter(o => o.status && col.statuses.includes(o.status as OrgStatus))
      .sort((a, b) => a.name.localeCompare(b.name))
      .map(o => ({ ...o, id: o.id })), // ensure `id` is a top-level property for svelte-dnd-action
  })))

  let hasAnyOrgs = $derived(organizations.length > 0)

  $effect(() => { loadOrganizations() })

  async function loadOrganizations() {
    loading = true
    const result = await forge.organizations.list({ limit: 500 })
    if (result.ok) {
      organizations = result.data.filter(o => o.status)
      loadEnrichmentData(organizations)
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load organizations'), type: 'error' })
    }
    loading = false
  }

  async function loadEnrichmentData(orgs: Organization[]) {
    const aliasMap = new Map<string, number>()
    const hqMap = new Map<string, string>()

    const promises = orgs.map(async (org) => {
      try {
        const [aliasRes, hq] = await Promise.all([
          fetch(`/api/organizations/${org.id}/aliases`), // raw until resu-mx/forge#127
          hqLocationLabel(forge, org.id),
        ])
        if (aliasRes.ok) {
          const aliasBody = await aliasRes.json()
          const aliases = aliasBody.data ?? []
          if (aliases.length > 0) {
            aliasMap.set(org.id, aliases.length)
          }
        }
        if (hq.ok && hq.data) hqMap.set(org.id, hq.data)
      } catch {
        // Silently skip enrichment failures
      }
    })

    await Promise.all(promises)
    aliasCountMap = aliasMap
    hqLocationMap = hqMap
  }

  async function handleDrop(columnKey: string, orgId: string) {
    if (!orgId) return // reorder within same column, ignore

    const newStatus = DROP_STATUS[columnKey]
    if (!newStatus) return

    // Check if org is already in this column's statuses
    const col = COLUMNS.find(c => c.key === columnKey)
    const org = organizations.find(o => o.id === orgId)
    if (!col || !org) return
    if (col.statuses.includes(org.status as OrgStatus)) return // same column, no-op

    // Optimistic update
    const oldStatus = org.status
    organizations = organizations.map(o =>
      o.id === orgId ? { ...o, status: newStatus } : o
    )

    // Persist
    const result = await forge.organizations.update(orgId, { status: newStatus })
    if (!result.ok) {
      // Revert on failure
      organizations = organizations.map(o =>
        o.id === orgId ? { ...o, status: oldStatus } : o
      )
      addToast({ message: friendlyError(result.error, 'Failed to update status'), type: 'error' })
    }
  }

  function handleCardClick(orgId: string) {
    detailOrgId = orgId
  }

  function handleDetailUpdate() {
    loadOrganizations()
    // If the detail org was removed from pipeline, close the modal
    // (loadOrganizations will filter it out; detailOrg will become null)
  }

  function handlePickerAdd() {
    loadOrganizations()
  }
</script>

<div class="kanban-page">
  <div class="kanban-header">
    <h2>Organization Pipeline</h2>
    <button class="btn-add" onclick={() => showPicker = true}>+ Add Organization</button>
  </div>

  {#if loading}
    <div class="board-loading">
      <LoadingSpinner size="lg" message="Loading pipeline..." />
    </div>
  {:else if !hasAnyOrgs}
    <div class="board-empty">
      <p>No organizations in the pipeline yet.</p>
      <p>Click <strong>+ Add Organization</strong> to start tracking.</p>
    </div>
  {:else}
    <div class="board-columns">
      {#each columnData as col, i (col.key)}
        <KanbanColumn
          label={col.label}
          accent={col.accent}
          items={col.items}
          collapsed={col.key === 'excluded' && !excludedExpanded}
          onToggleCollapse={col.key === 'excluded' ? () => { excludedExpanded = !excludedExpanded } : undefined}
          onDrop={(orgId) => handleDrop(col.key, orgId)}
          onCardClick={handleCardClick}
          {aliasCountMap}
          {hqLocationMap}
        />
      {/each}
    </div>
  {/if}
</div>

<OrgPickerModal
  open={showPicker}
  onclose={() => showPicker = false}
  onadd={handlePickerAdd}
/>

<OrgDetailModal
  org={detailOrg}
  onclose={() => detailOrgId = null}
  onupdate={handleDetailUpdate}
/>

<style>
  .kanban-page {
    display: flex;
    flex-direction: column;
    height: calc(100vh - var(--content-padding) * 2);
    margin: calc(-1 * var(--content-padding));
    background: var(--color-surface);
  }

  .kanban-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .kanban-header h2 {
    font-size: 1.15rem;
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin: 0;
  }

  .btn-add {
    padding: 0.4rem 0.85rem;
    background: var(--color-primary);
    color: var(--text-inverse);
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.82rem;
    font-weight: var(--font-medium);
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
    font-family: inherit;
  }

  .btn-add:hover { background: var(--color-primary-hover); }

  .board-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
  }

  .board-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-muted);
    font-size: 0.95rem;
    text-align: center;
    gap: 0.25rem;
  }

  .board-empty p {
    margin: 0;
  }

  .board-columns {
    display: flex;
    gap: var(--space-3);
    padding: var(--space-4);
    flex: 1;
    overflow-x: auto;
    align-items: stretch;
  }
</style>
