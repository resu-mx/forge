<!--
  ContactRelationSection.svelte -- From a contact: its linked organizations, job descriptions or
  resumes, with link and unlink. (ContactLinkSection is the other direction.)
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { Modal } from '$lib/components'
  import {
    RELATION_LABELS,
    linkRelation,
    listRelations,
    listTargets,
    unlinkRelation,
    type RelationKind,
    type RelationRow,
    type TargetOption,
  } from './contact-relations'

  let {
    kind,
    contactId,
    sectionTitle,
    relationships,
  }: {
    kind: RelationKind
    contactId: string
    sectionTitle: string
    relationships: { value: string; label: string }[]
  } = $props()

  const noun = $derived(RELATION_LABELS[kind].noun)
  const plural = $derived(RELATION_LABELS[kind].plural)

  let rows = $state<RelationRow[]>([])
  let options = $state<TargetOption[]>([])
  let showDialog = $state(false)
  let busy = $state(false)
  let pickedId = $state('')
  let pickedRelationship = $state('')

  $effect(() => {
    const id = contactId
    void refresh(id)
  })

  async function refresh(id = contactId) {
    const res = await listRelations(forge, kind, id)
    if (id !== contactId) return // another contact was selected meanwhile
    if (res.ok) rows = res.data
    else addToast({ type: 'error', message: friendlyError(res.error, `Failed to load linked ${plural}`) })
  }

  async function openDialog() {
    const res = await listTargets(forge, kind)
    if (!res.ok) {
      addToast({ type: 'error', message: friendlyError(res.error, `Failed to load ${plural}`) })
      return
    }
    options = res.data
    pickedId = ''
    pickedRelationship = relationships[0]?.value ?? ''
    showDialog = true
  }

  async function link() {
    if (!pickedId || !pickedRelationship) return
    busy = true
    const res = await linkRelation(forge, kind, contactId, pickedId, pickedRelationship)
    busy = false
    if (res.ok) {
      showDialog = false
      await refresh()
      addToast({ type: 'success', message: `${noun} linked` })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error, `Failed to link ${noun.toLowerCase()}`) })
    }
  }

  async function unlink(row: RelationRow) {
    const res = await unlinkRelation(forge, kind, contactId, row.target_id, row.relationship)
    if (res.ok) {
      await refresh()
      addToast({ type: 'success', message: `${noun} unlinked` })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error, `Failed to unlink ${noun.toLowerCase()}`) })
    }
  }

  function formatRelationship(rel: string): string {
    return rel.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
  }
</script>

<section class="link-section" aria-label={sectionTitle}>
  <div class="section-header">
    <span class="section-title">{sectionTitle}</span>
    <button class="btn btn-ghost btn-sm" onclick={openDialog} type="button">+ Link {noun}</button>
  </div>

  {#if rows.length === 0}
    <p class="empty-text">No {plural} linked.</p>
  {:else}
    <div class="linked-list">
      {#each rows as row (row.target_id + row.relationship)}
        <div class="linked-item">
          <span class="target-info">
            {row.label}
            {#if row.sublabel}<span class="target-sub"> -- {row.sublabel}</span>{/if}
          </span>
          <span class="relationship-badge">{formatRelationship(row.relationship)}</span>
          <button
            class="unlink-btn"
            onclick={() => unlink(row)}
            type="button"
            aria-label="Unlink {row.label}">&times;</button
          >
        </div>
      {/each}
    </div>
  {/if}
</section>

<Modal open={showDialog} onClose={() => (showDialog = false)} size="sm" title="Link {noun}">
  {#snippet body()}
    {#if options.length === 0}<p class="empty-text">No {plural} yet.</p>{/if}
    <div class="field">
      <label for="link-target">{noun}</label>
      <select id="link-target" bind:value={pickedId} disabled={options.length === 0}>
        <option value="" disabled>Select…</option>
        {#each options as o (o.id)}<option value={o.id}>{o.label}</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="link-relationship">Relationship</label>
      <select id="link-relationship" bind:value={pickedRelationship}>
        {#each relationships as rel (rel.value)}<option value={rel.value}>{rel.label}</option>{/each}
      </select>
    </div>
  {/snippet}
  {#snippet footer()}
    <button class="btn btn-ghost" onclick={() => (showDialog = false)} type="button">Cancel</button>
    <button class="btn btn-primary" onclick={link} disabled={busy || !pickedId || !pickedRelationship} type="button"
      >Link</button
    >
  {/snippet}
</Modal>

<style>
  .link-section {
    border-top: 1px solid var(--color-border);
    padding-top: 0.75rem;
    margin-top: 0.5rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .section-title {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .empty-text {
    font-size: 0.8rem;
    color: var(--text-faint);
    font-style: italic;
  }

  .linked-list {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .linked-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    background: var(--color-surface-raised);
    border-radius: 0.375rem;
    font-size: 0.85rem;
  }

  .target-info {
    flex: 1;
    color: var(--text-primary);
    font-weight: 500;
  }

  .target-sub {
    font-weight: 400;
    color: var(--text-muted);
  }

  .relationship-badge {
    font-size: 0.7rem;
    padding: 0.15em 0.5em;
    background: var(--color-tag-bg);
    color: var(--color-tag-text);
    border-radius: 999px;
    font-weight: 500;
    white-space: nowrap;
  }

  .unlink-btn {
    background: none;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0 0.25rem;
  }

  .unlink-btn:hover {
    color: var(--color-danger);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 1rem;
  }

  .field label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .field select {
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 0.375rem;
    font-size: 0.9rem;
    outline: none;
  }
</style>
