<!--
  ContactEditor.svelte -- Contact editor form with relationship sections.
-->
<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { ConfirmDialog } from '$lib/components'
  import ContactRelationSection from './ContactRelationSection.svelte'
  import type { ContactWithOrg, Organization } from '@forge/sdk'

  const ORG_RELATIONSHIPS = [
    { value: 'recruiter', label: 'Recruiter' },
    { value: 'hr', label: 'HR' },
    { value: 'referral', label: 'Referral' },
    { value: 'peer', label: 'Peer' },
    { value: 'manager', label: 'Manager' },
    { value: 'other', label: 'Other' },
  ]

  const JD_RELATIONSHIPS = [
    { value: 'hiring_manager', label: 'Hiring Manager' },
    { value: 'recruiter', label: 'Recruiter' },
    { value: 'interviewer', label: 'Interviewer' },
    { value: 'referral', label: 'Referral' },
    { value: 'other', label: 'Other' },
  ]

  const RESUME_RELATIONSHIPS = [
    { value: 'reference', label: 'Reference' },
    { value: 'recommender', label: 'Recommender' },
    { value: 'other', label: 'Other' },
  ]

  let {
    contact = null,
    organizations = [],
    createMode = false,
    oncreated,
    onupdated,
    ondeleted,
  }: {
    contact: ContactWithOrg | null
    organizations: Organization[]
    createMode?: boolean
    oncreated: (c: ContactWithOrg) => void
    onupdated: (c: ContactWithOrg) => void
    ondeleted: (id: string) => void
  } = $props()

  // Form state
  let name = $state('')
  let title = $state('')
  let organizationId = $state<string | null>(null)
  let email = $state('')
  let phone = $state('')
  let linkedin = $state('')
  let team = $state('')
  let dept = $state('')
  let notes = $state('')

  let saving = $state(false)
  let confirmDeleteOpen = $state(false)

  let isDirty = $derived.by(() => {
    if (createMode || !contact) return false
    return (
      name !== contact.name ||
      title !== (contact.title ?? '') ||
      organizationId !== (contact.organization_id ?? null) ||
      email !== (contact.email ?? '') ||
      phone !== (contact.phone ?? '') ||
      linkedin !== (contact.linkedin ?? '') ||
      team !== (contact.team ?? '') ||
      dept !== (contact.dept ?? '') ||
      notes !== (contact.notes ?? '')
    )
  })

  $effect(() => {
    if (contact && !createMode) {
      name = contact.name
      title = contact.title ?? ''
      organizationId = contact.organization_id ?? null
      email = contact.email ?? ''
      phone = contact.phone ?? ''
      linkedin = contact.linkedin ?? ''
      team = contact.team ?? ''
      dept = contact.dept ?? ''
      notes = contact.notes ?? ''
    } else if (createMode) {
      name = ''
      title = ''
      organizationId = null
      email = ''
      phone = ''
      linkedin = ''
      team = ''
      dept = ''
      notes = ''
    }
  })

  async function handleSave() {
    if (!name.trim()) {
      addToast({ type: 'error', message: 'Name is required' })
      return
    }

    saving = true
    const payload = {
      name: name.trim(),
      title: title.trim() || null,
      organization_id: organizationId,
      email: email.trim() || null,
      phone: phone.trim() || null,
      linkedin: linkedin.trim() || null,
      team: team.trim() || null,
      dept: dept.trim() || null,
      notes: notes.trim() || null,
    }

    if (createMode) {
      const res = await forge.contacts.create(payload as any)
      if (res.ok) {
        oncreated(res.data)
        addToast({ type: 'success', message: 'Contact created' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    } else if (contact) {
      const res = await forge.contacts.update(contact.id, payload)
      if (res.ok) {
        onupdated(res.data)
        addToast({ type: 'success', message: 'Contact updated' })
      } else {
        addToast({ type: 'error', message: friendlyError(res.error) })
      }
    }
    saving = false
  }

  async function handleDelete() {
    if (!contact) return
    const res = await forge.contacts.delete(contact.id)
    if (res.ok) {
      ondeleted(contact.id)
      addToast({ type: 'success', message: 'Contact deleted' })
    } else {
      addToast({ type: 'error', message: friendlyError(res.error) })
    }
    confirmDeleteOpen = false
  }
</script>

<div class="editor">
  <div class="field">
    <label for="contact-name">Name <span class="required">*</span></label>
    <input id="contact-name" type="text" bind:value={name} placeholder="Full name" />
  </div>

  <div class="field">
    <label for="contact-title">Title</label>
    <input id="contact-title" type="text" bind:value={title} placeholder="Job title" />
  </div>

  <div class="field">
    <label for="contact-org">Organization</label>
    <select id="contact-org" bind:value={organizationId}>
      <option value={null}>None</option>
      {#each [...organizations].sort((a, b) => a.name.localeCompare(b.name)) as org (org.id)}
        <option value={org.id}>{org.name}</option>
      {/each}
    </select>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="contact-email">Email</label>
      <input id="contact-email" type="email" bind:value={email} placeholder="email@example.com" />
    </div>
    <div class="field half">
      <label for="contact-phone">Phone</label>
      <input id="contact-phone" type="tel" bind:value={phone} placeholder="+1-555-0123" />
    </div>
  </div>

  <div class="field">
    <label for="contact-linkedin">LinkedIn</label>
    <div class="url-field">
      <input id="contact-linkedin" type="url" bind:value={linkedin} placeholder="https://linkedin.com/in/..." />
      {#if linkedin.trim() && !createMode}
        <a href={linkedin} target="_blank" rel="noopener noreferrer" class="url-link">
          Open
        </a>
      {/if}
    </div>
  </div>

  <div class="field-row">
    <div class="field half">
      <label for="contact-team">Team</label>
      <input id="contact-team" type="text" bind:value={team} placeholder="Platform Security" />
    </div>
    <div class="field half">
      <label for="contact-dept">Dept</label>
      <input id="contact-dept" type="text" bind:value={dept} placeholder="Engineering" />
    </div>
  </div>

  <div class="field">
    <label for="contact-notes">Notes</label>
    <textarea id="contact-notes" bind:value={notes} placeholder="Your private notes..." rows="4"></textarea>
  </div>

  {#if !createMode && contact}
    <ContactRelationSection
      kind="organization"
      contactId={contact.id}
      sectionTitle="Linked Organizations"
      relationships={ORG_RELATIONSHIPS}
    />

    <ContactRelationSection
      kind="job_description"
      contactId={contact.id}
      sectionTitle="Linked Job Descriptions"
      relationships={JD_RELATIONSHIPS}
    />

    <ContactRelationSection
      kind="resume"
      contactId={contact.id}
      sectionTitle="Linked Resumes"
      relationships={RESUME_RELATIONSHIPS}
    />
  {/if}

  <div class="actions">
    <button
      class="btn-primary"
      onclick={handleSave}
      disabled={saving || (!createMode && !isDirty)}
    >
      {saving ? 'Saving...' : createMode ? 'Create' : 'Save'}
    </button>
    {#if !createMode && contact}
      <button
        class="btn-danger"
        onclick={() => (confirmDeleteOpen = true)}
        type="button"
      >
        Delete
      </button>
    {/if}
  </div>
</div>

<ConfirmDialog
  open={confirmDeleteOpen}
  title="Delete Contact"
  message="Are you sure you want to delete this contact? This cannot be undone."
  onconfirm={handleDelete}
  oncancel={() => (confirmDeleteOpen = false)}
/>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field-row {
    display: flex;
    gap: 1rem;
  }

  .half {
    flex: 1;
  }

  label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .required {
    color: var(--color-danger);
  }

  input[type="text"],
  input[type="email"],
  input[type="tel"],
  input[type="url"],
  select,
  textarea {
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--color-border-strong);
    border-radius: 0.375rem;
    font-size: 0.9rem;
    outline: none;
    font-family: inherit;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--color-info);
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .url-field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .url-field input {
    flex: 1;
  }

  .url-link {
    font-size: 0.8rem;
    color: var(--color-info);
    text-decoration: none;
    white-space: nowrap;
  }

  .url-link:hover {
    text-decoration: underline;
  }

  .actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.5rem;
    border-top: 1px solid var(--color-border);
  }

</style>
