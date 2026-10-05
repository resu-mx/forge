<!--
  JDSkillExtraction.svelte -- "Extract Skills" button + review panel.

  Extraction state is ephemeral (component state only). Navigating away
  clears suggestions. Re-clicking "Extract Skills" replaces any existing
  suggestions with a new extraction.

  Accepting a skill calls the existing POST /api/job-descriptions/:id/skills
  endpoint. The skill is created on accept, not on extract, preventing
  orphaned skills from rejected suggestions.

  "Accept All Remaining" accepts skills sequentially (not in parallel)
  to avoid race conditions in skill auto-creation.

  onSkillsChanged callback triggers a refresh of the parent's jdSkills list
  so the alreadyLinkedExtracted derived state updates correctly after
  accepting a skill.
-->
<script lang="ts">
  import type { ForgeClient } from '@forge/sdk'
  import type { ExtractedSkill } from '@forge/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { friendlyError } from '$lib/sdk'
  import { acceptFailureMessage, acceptSkills } from './extraction-accept'
  import ExtractedSkillCard from './ExtractedSkillCard.svelte'

  interface LinkedSkill {
    id: string
    name: string
  }

  let { jdId, jdSkills, forge, onSkillsChanged }: {
    jdId: string
    jdSkills: LinkedSkill[]
    forge: ForgeClient
    onSkillsChanged: () => void
  } = $props()

  let extractedSkills = $state<ExtractedSkill[]>([])
  let extracting = $state(false)
  let showPanel = $state(false)
  let errorMessage = $state('')

  // Track which extracted skills have been accepted or dismissed
  let acceptedNames = $state<Set<string>>(new Set())
  let dismissedNames = $state<Set<string>>(new Set())

  // Editable names (user can rename before accepting)
  let editedNames = $state<Map<string, string>>(new Map())

  // Derived: skills available for review (not accepted, not dismissed, not already linked)
  let reviewableSkills = $derived(
    extractedSkills
      .filter(s => !acceptedNames.has(s.name) && !dismissedNames.has(s.name))
      .filter(s => !jdSkills.some(js =>
        js.name.toLowerCase() === s.name.toLowerCase()
      ))
      .sort((a, b) => b.confidence - a.confidence)
  )

  // Derived: skills from extraction that are already linked to this JD
  let alreadyLinkedExtracted = $derived(
    extractedSkills.filter(s =>
      jdSkills.some(js =>
        js.name.toLowerCase() === s.name.toLowerCase()
      )
    )
  )

  let acceptingAll = $state(false)

  async function handleExtract() {
    addToast({ message: 'Skill extraction temporarily disabled — use MCP tools (forge_extract_jd_skills)', type: 'info' })
    return
  }

  async function acceptAndReport(skills: ExtractedSkill[]) {
    const outcome = await acceptSkills(
      skills.map((s) => ({ key: s.name, name: editedNames.get(s.name) ?? s.name })),
      (name) => forge.jobDescriptions.addSkill(jdId, { name }),
      (e) => friendlyError(e),
    )
    if (outcome.accepted.length > 0) {
      acceptedNames = new Set([...acceptedNames, ...outcome.accepted])
      onSkillsChanged()
    }
    // Failed suggestions stay out of acceptedNames, so they remain reviewable.
    errorMessage = acceptFailureMessage(outcome.failed) ?? ''
  }

  async function handleAccept(skill: ExtractedSkill) {
    await acceptAndReport([skill])
  }

  // Sequential accept-all (see acceptSkills), with one message for all failures.
  async function handleAcceptAll() {
    acceptingAll = true
    try {
      await acceptAndReport([...reviewableSkills]) // snapshot: reviewableSkills is $derived
    } finally {
      acceptingAll = false
    }
  }

  function handleDismiss(skill: ExtractedSkill) {
    dismissedNames = new Set([...dismissedNames, skill.name])
  }

  function handleDismissAll() {
    for (const skill of reviewableSkills) {
      dismissedNames = new Set([...dismissedNames, skill.name])
    }
    showPanel = false
  }

  function handleEdit(skill: ExtractedSkill, newName: string) {
    editedNames = new Map([...editedNames, [skill.name, newName]])
  }
</script>

<div class="jd-skill-extraction">
  <button
    class="extract-btn"
    onclick={handleExtract}
    disabled={extracting}
  >
    {#if extracting}
      Extracting...
    {:else}
      Extract Skills
    {/if}
  </button>

  {#if errorMessage}
    <p class="error-message">{errorMessage}</p>
  {/if}

  {#if showPanel && extractedSkills.length > 0}
    <div class="extraction-panel">
      <h4>Extracted Skills</h4>

      {#if alreadyLinkedExtracted.length > 0}
        <div class="already-linked-section">
          {#each alreadyLinkedExtracted as skill (skill.name)}
            <ExtractedSkillCard
              {skill}
              isLinked={true}
              onaccept={() => {}}
              ondismiss={() => {}}
              onedit={() => {}}
            />
          {/each}
          <p class="already-linked-note muted">
            {alreadyLinkedExtracted.length} skill{alreadyLinkedExtracted.length === 1 ? '' : 's'} already linked (skipped)
          </p>
        </div>
      {/if}

      {#if reviewableSkills.length > 0}
        <div class="reviewable-section">
          {#each reviewableSkills as skill (skill.name)}
            <ExtractedSkillCard
              {skill}
              isLinked={false}
              onaccept={() => handleAccept(skill)}
              ondismiss={() => handleDismiss(skill)}
              onedit={(newName) => handleEdit(skill, newName)}
            />
          {/each}
        </div>

        <div class="bulk-actions">
          <button
            class="accept-all-btn"
            onclick={handleAcceptAll}
            disabled={acceptingAll}
          >
            {#if acceptingAll}
              Accepting...
            {:else}
              Accept All Remaining ({reviewableSkills.length})
            {/if}
          </button>
          <button class="dismiss-all-btn" onclick={handleDismissAll}>
            Dismiss All
          </button>
        </div>
      {:else if acceptedNames.size > 0 || dismissedNames.size > 0}
        <p class="muted all-reviewed">All extracted skills have been reviewed.</p>
      {/if}
    </div>
  {:else if showPanel && extractedSkills.length === 0}
    <div class="extraction-panel">
      <p class="muted">
        No skills were extracted from this job description. The text may not
        contain identifiable technical requirements.
      </p>
    </div>
  {/if}
</div>

<style>
  .jd-skill-extraction {
    margin-top: 12px;
  }

  .extract-btn {
    font-size: 0.875rem;
    padding: 4px 12px;
    border: 1px solid var(--color-border-strong, #d1d5db);
    border-radius: 4px;
    background: var(--color-surface, white);
    cursor: pointer;
  }

  .extract-btn:hover:not(:disabled) {
    background: var(--color-ghost, #f9fafb);
  }

  .extract-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-message {
    color: var(--color-danger, #dc2626);
    font-size: 0.875rem;
    margin-top: 4px;
  }

  .extraction-panel {
    margin-top: 8px;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 6px;
    padding: 8px;
  }

  .extraction-panel h4 {
    font-size: 0.875rem;
    margin: 0 0 8px 0;
  }

  .already-linked-section {
    margin-bottom: 8px;
    border-bottom: 1px solid var(--color-border, #f3f4f6);
    padding-bottom: 8px;
  }

  .already-linked-note {
    font-size: 0.75rem;
    margin: 4px 0 0 0;
    color: var(--text-muted, #6b7280);
  }

  .all-reviewed {
    color: var(--text-muted, #6b7280);
    font-size: 0.875rem;
    margin: 8px 0 0 0;
  }

  .bulk-actions {
    display: flex;
    justify-content: space-between;
    padding-top: 8px;
    border-top: 1px solid var(--color-border, #f3f4f6);
    margin-top: 8px;
  }

  .accept-all-btn {
    font-size: 0.75rem;
    padding: 4px 12px;
    border: 1px solid var(--color-success, #22c55e);
    border-radius: 4px;
    background: var(--color-success-subtle, #f0fdf4);
    color: var(--color-success-text, #16a34a);
    cursor: pointer;
  }

  .accept-all-btn:hover:not(:disabled) {
    background: #dcfce7;
  }

  .accept-all-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .dismiss-all-btn {
    font-size: 0.75rem;
    padding: 4px 12px;
    border: 1px solid var(--color-border-strong, #d1d5db);
    border-radius: 4px;
    background: var(--color-surface, white);
    color: var(--text-muted, #6b7280);
    cursor: pointer;
  }

  .dismiss-all-btn:hover {
    background: var(--color-danger-subtle, #fef2f2);
    color: var(--color-danger, #dc2626);
    border-color: #fecaca;
  }
</style>
