<script lang="ts">
  import { forge, friendlyError } from '$lib/sdk'
  import { addToast } from '$lib/stores/toast.svelte'
  import { onForgeChanged } from '$lib/forge-changed'
  import { DERIVE_VIA_AGENT } from '$lib/derive-message'
  import { StatusBadge, LoadingSpinner, EmptyState, ConfirmDialog, SplitPanel, ListPanelHeader, EmptyPanel } from '$lib/components'
  import OrgCombobox from '$lib/components/OrgCombobox.svelte'
  import type { Source, Organization, Skill, ClearanceLevel, ClearancePolygraph, ClearanceStatus, ClearanceType, ClearanceAccessProgram } from '@forge/sdk'
  import {
    CLEARANCE_LEVELS,
    CLEARANCE_POLYGRAPHS,
    CLEARANCE_STATUSES,
    CLEARANCE_TYPES,
    CLEARANCE_ACCESS_PROGRAMS,
    CLEARANCE_LEVEL_LABELS,
    CLEARANCE_POLYGRAPH_LABELS,
    CLEARANCE_ACCESS_PROGRAM_LABELS,
  } from '@forge/sdk'

  /** Optional: lock this view to a single source type (hides the type filter tabs). */
  let { sourceTypeFilter = undefined }: { sourceTypeFilter?: string } = $props()

  const SOURCE_TABS = [
    { value: 'all', label: 'All' },
    { value: 'role', label: 'Roles' },
    { value: 'project', label: 'Projects' },
    { value: 'education', label: 'Education' },
    { value: 'clearance', label: 'Clearances' },
    { value: 'general', label: 'General' },
  ]

  let sources = $state<Source[]>([])
  let organizations = $state<Organization[]>([])
  let activeTab = $state(sourceTypeFilter ?? 'all')
  let selectedId = $state<string | null>(null)
  let loading = $state(true)
  let editing = $state(false)
  let saving = $state(false)
  let deriving = $state(false)
  let confirmDeleteOpen = $state(false)

  // Base form fields
  let formTitle = $state('')
  let formDescription = $state('')
  let formSourceType = $state<string>('general')
  let formNotes = $state('')

  // Role extension fields
  let formOrgId = $state<string | null>(null)
  let formStartDate = $state('')
  let formEndDate = $state('')
  let formIsCurrent = $state(false)
  let formWorkArrangement = $state('')
  let formBaseSalary = $state<number | null>(null)
  let formTotalCompNotes = $state('')

  // Education extension fields
  let formEducationType = $state('degree')
  let formEduOrgId = $state<string | null>(null)
  let formField = $state('')
  let formIsInProgress = $state(false)
  let formCredentialId = $state('')
  let formExpirationDate = $state('')
  let formUrl = $state('')

  // Campus
  let formCampusId = $state<string | null>(null)
  let campuses = $state<{ id: string; name: string; modality: string; city: string | null; state: string | null }[]>([])
  let showCampusModal = $state(false)
  let newCampusName = $state('')
  let newCampusModality = $state('in_person')
  let newCampusCity = $state('')
  let newCampusState = $state('')
  let creatingCampus = $state(false)

  // Source skills
  let sourceSkills = $state<Skill[]>([])
  let allSkills = $state<Skill[]>([])
  let skillSearchQuery = $state('')
  let showSkillDropdown = $state(false)
  let addingSkill = $state(false)

  // Quick-create org modal
  let showOrgModal = $state(false)
  let newOrgName = $state('')
  let newOrgType = $state('education')
  let newOrgWebsite = $state('')
  let creatingOrg = $state(false)

  // Education sub-type fields
  let formDegreeLevel = $state<string>('bachelors')
  let formDegreeType = $state('')
  let formGpa = $state('')
  let formLocation = $state('')
  let formEduDescription = $state('')

  // Project extension fields
  let formIsPersonal = $state(false)
  let formOpenSource = $state(false)
  let formProjectUrl = $state('')

  // Presentation extension fields
  let formVenue = $state('')
  let formPresentationType = $state('conference_talk')
  let formPresentationUrl = $state('')
  let formCoauthors = $state('')

  // Clearance extension fields
  let formLevel = $state<ClearanceLevel>('secret')
  let formPolygraph = $state<ClearancePolygraph | ''>('')
  let formClearanceStatus = $state<ClearanceStatus>('active')
  let formClearanceType = $state<ClearanceType>('personnel')
  let formSponsorOrgId = $state('')
  let formContinuousInvestigation = $state(false)
  let formAccessPrograms = $state<ClearanceAccessProgram[]>([])

  // Education grouping
  type GroupMode = 'flat' | 'by_type' | 'by_issuer'
  const GROUP_OPTIONS: { value: GroupMode; label: string }[] = [
    { value: 'flat', label: 'Flat' },
    { value: 'by_type', label: 'By Type' },
    { value: 'by_issuer', label: 'By Issuer' },
  ]
  let eduGroupBy = $state<GroupMode>('by_type')
  let collapsedGroups = $state<Record<string, boolean>>({})

  function toggleGroupCollapse(key: string) {
    collapsedGroups[key] = !collapsedGroups[key]
  }

  const EDU_TYPE_LABELS: Record<string, string> = {
    degree: 'Degrees',
    course: 'Courses',
    self_taught: 'Self-Taught',
  }

  let filteredSources = $derived(
    activeTab === 'all'
      ? sources
      : sources.filter(s => s.source_type === activeTab)
  )

  /** Group education sources into labeled sections */
  let groupedEducation = $derived.by(() => {
    if (activeTab !== 'education' || eduGroupBy === 'flat') return null
    const eduSources = sources.filter(s => s.source_type === 'education')
    const groups = new Map<string, { label: string; sources: Source[] }>()

    for (const s of eduSources) {
      let key: string
      let label: string

      if (eduGroupBy === 'by_type') {
        key = s.education?.education_type ?? 'unknown'
        label = EDU_TYPE_LABELS[key] ?? key
      } else {
        // by_issuer — group by organization
        const orgId = s.education?.organization_id
        if (orgId) {
          key = orgId
          label = getOrgName(orgId)
        } else {
          key = '_unknown'
          label = 'No Organization'
        }
      }

      if (!groups.has(key)) {
        groups.set(key, { label, sources: [] })
      }
      groups.get(key)!.sources.push(s)
    }

    // Sort groups: for by_type use a fixed order, otherwise alphabetical
    const entries = [...groups.entries()]
    if (eduGroupBy === 'by_type') {
      const order = ['degree', 'course', 'self_taught']
      entries.sort((a, b) => order.indexOf(a[0]) - order.indexOf(b[0]))
    } else {
      entries.sort((a, b) => a[1].label.localeCompare(b[1].label))
    }

    return entries.map(([key, group]) => ({ key, ...group }))
  })

  let selectedSource = $derived(sources.find(s => s.id === selectedId) ?? null)

  $effect(() => {
    loadSources()
    loadOrganizations()
    loadAllSkills()
  })

  // An agent (or another part of the page) changed data: refresh the list without the spinner.
  $effect(() => onForgeChanged(() => loadSources(true)))

  $effect(() => {
    if (selectedSource && !editing) {
      populateFormFromSource(selectedSource)
    }
  })

  $effect(() => {
    if (formIsPersonal) {
      formOrgId = null
    }
  })

  // Load campuses when education org changes
  $effect(() => {
    if (formEduOrgId) {
      loadCampuses(formEduOrgId)
    } else {
      campuses = []
      formCampusId = null
    }
  })

  async function loadCampuses(orgId: string) {
    const res = await fetch(`/api/organizations/${orgId}/campuses`)
    if (res.ok) {
      const body = await res.json()
      campuses = body.data ?? []
    } else {
      campuses = []
    }
  }

  function openCampusModal() {
    newCampusName = ''
    newCampusModality = 'in_person'
    newCampusCity = ''
    newCampusState = ''
    showCampusModal = true
  }

  async function createCampusAndSelect() {
    if (!newCampusName.trim() || !formEduOrgId) return
    creatingCampus = true
    const res = await fetch(`/api/organizations/${formEduOrgId}/campuses`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: newCampusName.trim(),
        modality: newCampusModality,
        city: newCampusCity.trim() || undefined,
        state: newCampusState.trim() || undefined,
      }),
    })
    if (res.ok) {
      const body = await res.json()
      campuses = [...campuses, body.data]
      formCampusId = body.data.id
      showCampusModal = false
      addToast({ message: `Campus "${body.data.name}" created.`, type: 'success' })
    } else {
      addToast({ message: 'Failed to create campus.', type: 'error' })
    }
    creatingCampus = false
  }

  // Education type change handler — resets irrelevant sub-fields when user
  // switches education type via the dropdown. NOT an $effect (which would
  // fire during populateFormFromSource and clobber loaded values).
  function handleEducationTypeChange(newType: string) {
    formEducationType = newType
    if (newType !== 'degree') {
      formDegreeLevel = ''
      formDegreeType = ''
      formGpa = ''
    }
    if (newType !== 'degree' && newType !== 'course') {
      formLocation = ''
    }
    // Reset org selection when type changes (different org filter per type)
    formEduOrgId = null
  }

  async function loadSources(quiet = false) {
    if (!quiet) loading = true
    const result = await forge.sources.list({ limit: 500 })
    if (result.ok) {
      sources = result.data
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to load sources'), type: 'error' })
    }
    loading = false
  }

  async function loadOrganizations() {
    const orgResult = await forge.organizations.list({ limit: 500 })
    if (orgResult.ok) {
      // Load ALL orgs — education dropdowns filter by tags via eduFilteredOrgs,
      // role/project dropdowns filter by workedOrgIds via roleFilteredOrgs
      organizations = orgResult.data
    }
  }

  function getOrgName(id: string): string {
    return organizations.find(o => o.id === id)?.name ?? 'Unknown'
  }

  function populateFormFromSource(source: Source) {
    formTitle = source.title
    formDescription = source.description
    formSourceType = source.source_type
    formNotes = source.notes ?? ''
    // Reset all extension fields
    formOrgId = null
    formStartDate = ''
    formEndDate = ''
    formIsCurrent = false
    formWorkArrangement = ''
    formBaseSalary = null
    formTotalCompNotes = ''
    formEducationType = 'degree'
    formEduOrgId = null
    formField = ''
    formIsInProgress = false
    formCredentialId = ''
    formExpirationDate = ''
    // formIssuingBody removed — use formEduOrgId
    formUrl = ''
    formDegreeLevel = ''
    formDegreeType = ''
    formGpa = ''
    formLocation = ''
    formEduDescription = ''
    formIsPersonal = false
    formOpenSource = false
    formProjectUrl = ''
    formVenue = ''
    formPresentationType = 'conference_talk'
    formPresentationUrl = ''
    formCoauthors = ''
    formLevel = 'secret'
    formPolygraph = ''
    formClearanceStatus = 'active'
    formClearanceType = 'personnel'
    formSponsorOrgId = ''
    formContinuousInvestigation = false
    formAccessPrograms = []

    if (source.source_type === 'role' && source.role) {
      formOrgId = source.role.organization_id ?? null
      formStartDate = source.role.start_date ?? ''
      formEndDate = source.role.end_date ?? ''
      formIsCurrent = !!source.role.is_current
      formWorkArrangement = source.role.work_arrangement ?? ''
      formBaseSalary = source.role.base_salary ?? null
      formTotalCompNotes = source.role.total_comp_notes ?? ''
    } else if (source.source_type === 'education' && source.education) {
      formEducationType = source.education.education_type
      formEduOrgId = source.education.organization_id ?? null
      formCampusId = source.education.campus_id ?? null
      formField = source.education.field ?? ''
      formStartDate = source.education.start_date ?? ''
      formEndDate = source.education.end_date ?? ''
      formIsInProgress = !!source.education.is_in_progress
      formCredentialId = source.education.credential_id ?? ''
      formExpirationDate = source.education.expiration_date ?? ''
      // issuing_body now comes from organization_id
      formUrl = source.education.url ?? ''
      formDegreeLevel = source.education.degree_level ?? ''
      formDegreeType = source.education.degree_type ?? ''
      formGpa = source.education.gpa ?? ''
      formLocation = source.education.location ?? ''
      formEduDescription = source.education.edu_description ?? ''
    } else if (source.source_type === 'project' && source.project) {
      formOrgId = source.project.organization_id ?? null
      formIsPersonal = !!source.project.is_personal
      formOpenSource = !!(source.project as any).open_source
      formProjectUrl = source.project.url ?? ''
      formStartDate = source.project.start_date ?? ''
      formEndDate = source.project.end_date ?? ''
    } else if (source.source_type === 'presentation' && source.presentation) {
      formVenue = source.presentation.venue ?? ''
      formPresentationType = source.presentation.presentation_type ?? 'conference_talk'
      formPresentationUrl = source.presentation.url ?? ''
      formCoauthors = source.presentation.coauthors ?? ''
    } else if (source.source_type === 'clearance' && source.clearance) {
      formLevel = source.clearance.level
      formPolygraph = source.clearance.polygraph ?? ''
      formClearanceStatus = source.clearance.status
      formClearanceType = source.clearance.type
      formSponsorOrgId = source.clearance.sponsor_organization_id ?? ''
      formContinuousInvestigation = !!source.clearance.continuous_investigation
      formAccessPrograms = source.clearance.access_programs ?? []
    }
  }

  function startNew() {
    selectedId = null
    editing = true
    formTitle = ''
    formDescription = ''
    formSourceType = sourceTypeFilter ?? 'general'
    formNotes = ''
    formOrgId = null
    formStartDate = ''
    formEndDate = ''
    formIsCurrent = false
    formWorkArrangement = ''
    formBaseSalary = null
    formTotalCompNotes = ''
    formEducationType = 'degree'
    formEduOrgId = null
    formField = ''
    formIsInProgress = false
    formCredentialId = ''
    formExpirationDate = ''
    // formIssuingBody removed — use formEduOrgId
    formUrl = ''
    formDegreeLevel = ''
    formDegreeType = ''
    formGpa = ''
    formLocation = ''
    formEduDescription = ''
    formIsPersonal = false
    formOpenSource = false
    formProjectUrl = ''
    formVenue = ''
    formPresentationType = 'conference_talk'
    formPresentationUrl = ''
    formCoauthors = ''
    formLevel = 'secret'
    formPolygraph = ''
    formClearanceStatus = 'active'
    formClearanceType = 'personnel'
    formSponsorOrgId = ''
    formContinuousInvestigation = false
    formAccessPrograms = []
  }

  async function selectSource(id: string) {
    editing = false
    selectedId = id
    // Fetch full source with extension data (list endpoint doesn't include extensions)
    const result = await forge.sources.get(id)
    if (result.ok) {
      // Replace the list entry with the full source (includes education/role/project/clearance)
      sources = sources.map(s => s.id === id ? result.data : s)
    }
    // Load skills for this source
    loadSourceSkills(id)
  }

  async function saveSource() {
    if (!formTitle.trim() || !formDescription.trim()) {
      addToast({ message: 'Title and description are required.', type: 'error' })
      return
    }

    saving = true
    const basePayload: Record<string, unknown> = {
      title: formTitle.trim(),
      description: formDescription.trim(),
      notes: formNotes || undefined,
    }

    // Build extension payload based on source_type
    if (formSourceType === 'role') {
      basePayload.role = {
        organization_id: formOrgId,
        start_date: formStartDate || undefined,
        end_date: formEndDate || undefined,
        is_current: formIsCurrent,
        work_arrangement: formWorkArrangement || undefined,
        base_salary: formBaseSalary,
        total_comp_notes: formTotalCompNotes || undefined,
      }
    } else if (formSourceType === 'education') {
      basePayload.education = {
        education_type: formEducationType,
        education_organization_id: formEduOrgId || undefined,
        campus_id: formCampusId || undefined,
        field: formField || undefined,
        start_date: formStartDate || undefined,
        end_date: formEndDate || undefined,
        is_in_progress: formIsInProgress,
        credential_id: formCredentialId || undefined,
        expiration_date: formExpirationDate || undefined,
        // issuing_body replaced by education_organization_id
        url: formUrl || undefined,
        degree_level: formDegreeLevel || undefined,
        degree_type: formDegreeType || undefined,
        gpa: formGpa || undefined,
        location: formLocation || undefined,
        edu_description: formEduDescription || undefined,
      }
    } else if (formSourceType === 'project') {
      basePayload.project = {
        organization_id: formOrgId,
        is_personal: formIsPersonal,
        open_source: formOpenSource,
        url: formProjectUrl || undefined,
        start_date: formStartDate || undefined,
        end_date: formEndDate || undefined,
      }
    } else if (formSourceType === 'presentation') {
      basePayload.presentation = {
        venue: formVenue || undefined,
        presentation_type: formPresentationType || undefined,
        url: formPresentationUrl || undefined,
        coauthors: formCoauthors || undefined,
      }
    } else if (formSourceType === 'clearance') {
      basePayload.clearance = {
        level: formLevel,
        polygraph: formPolygraph || undefined,
        status: formClearanceStatus,
        type: formClearanceType,
        sponsor_organization_id: formSponsorOrgId || undefined,
        continuous_investigation: formContinuousInvestigation ? 1 : 0,
        access_programs: formAccessPrograms,
      }
    }

    if (editing) {
      const result = await forge.sources.create({
        ...basePayload,
        source_type: formSourceType as any,
      } as any)
      if (result.ok) {
        sources = [...sources, result.data]
        selectedId = result.data.id
        editing = false
        addToast({ message: 'Source created.', type: 'success' })
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to create source'), type: 'error' })
      }
    } else if (selectedId) {
      const result = await forge.sources.update(selectedId, basePayload as any)
      if (result.ok) {
        sources = sources.map(s => s.id === selectedId ? result.data : s)
        addToast({ message: 'Source updated.', type: 'success' })
      } else {
        addToast({ message: friendlyError(result.error, 'Failed to update source'), type: 'error' })
      }
    }
    saving = false
  }

  async function deleteSource() {
    if (!selectedId) return
    confirmDeleteOpen = false

    const id = selectedId
    const result = await forge.sources.delete(id)
    if (result.ok) {
      sources = sources.filter(s => s.id !== id)
      selectedId = null
      editing = false
      addToast({ message: 'Source deleted.', type: 'success' })
    } else {
      addToast({ message: `Failed to delete source: ${result.error.message}`, type: 'error' })
    }
  }

  async function deriveBullets() {
    addToast({ message: DERIVE_VIA_AGENT, type: 'info' })
  }

  /**
   * All organizations for role/project dropdowns, sorted with "worked at"
   * orgs first.
   *
   * The original design filtered to ONLY orgs the user had worked at. That
   * created a chicken-and-egg problem: you couldn't link an org you hadn't
   * already linked. And once one org was linked, only that one showed up.
   *
   * Fix: show ALL orgs always, but sort "worked at" orgs to the top so the
   * user's most common picks are first. New orgs are always reachable by
   * scrolling. This matches the eduFilteredOrgs approach (show everything,
   * prioritize relevant ones).
   */
  let roleFilteredOrgs = $derived.by(() => {
    const workedAtIds = new Set(
      sources
        .filter(s => s.source_type === 'role' || s.source_type === 'project')
        .map(s => s.role?.organization_id ?? s.project?.organization_id)
        .filter((id): id is string => id !== null && id !== undefined)
    )
    return [...organizations].sort((a, b) => {
      const aWorked = workedAtIds.has(a.id) ? 0 : 1
      const bWorked = workedAtIds.has(b.id) ? 0 : 1
      if (aWorked !== bWorked) return aWorked - bWorked
      return a.name.localeCompare(b.name)
    })
  })

  /**
   * Filter orgs by relevant tags for the current education type + cert subtype.
   *
   * Graceful degradation: an org passes the filter when it has at least one
   * matching tag OR it has no tags at all. Strict tag filtering hid
   * legitimate educational institutions (e.g. SANS, GIAC) that simply
   * haven't been tagged yet. Treating untagged orgs as "unknown, show
   * anyway" makes the picker usable today while still preferring the
   * properly-tagged ones as tag data fills in.
   */
  function matchesOrFallsBack(org: Organization, allowedTags: string[]): boolean {
    const tags = org.tags ?? []
    if (tags.length === 0) return true
    return tags.some(t => allowedTags.includes(t))
  }

  let eduFilteredOrgs = $derived.by(() => {
    if (formEducationType === 'degree') {
      return organizations.filter(o => matchesOrFallsBack(o, ['university', 'school']))
    }
    if (formEducationType === 'course') {
      return organizations.filter(o => matchesOrFallsBack(o, ['university', 'platform', 'conference', 'company']))
    }
    // self_taught — show all
    return organizations
  })

  // Infer default tags for new org based on education context
  let newOrgTags = $state<string[]>([])

  function openOrgModal() {
    newOrgName = ''
    newOrgWebsite = ''
    // Infer org_type and tags from education context
    if (formEducationType === 'degree') {
      newOrgType = 'education'
      newOrgTags = ['university']
    } else if (formEducationType === 'course') {
      newOrgType = 'education'
      newOrgTags = ['platform']
    } else {
      newOrgType = 'company'
      newOrgTags = ['company']
    }
    showOrgModal = true
  }

  async function createOrgAndSelect() {
    if (!newOrgName.trim()) return
    creatingOrg = true
    const result = await forge.organizations.create({
      name: newOrgName.trim(),
      org_type: newOrgType,
      website: newOrgWebsite.trim() || undefined,
      tags: newOrgTags,
    } as any)
    if (result.ok) {
      organizations = [...organizations, result.data]
      formEduOrgId = result.data.id
      showOrgModal = false
      addToast({ message: `Organization "${result.data.name}" created.`, type: 'success' })
    } else {
      addToast({ message: friendlyError(result.error, 'Failed to create organization'), type: 'error' })
    }
    creatingOrg = false
  }

  // ── Source Skills ──────────────────────────────────────────────────

  async function loadAllSkills() {
    const result = await forge.skills.list()
    if (result.ok) {
      allSkills = result.data
    }
  }

  async function loadSourceSkills(sourceId: string) {
    const res = await fetch(`/api/sources/${sourceId}/skills`)
    if (res.ok) {
      const body = await res.json()
      sourceSkills = body.data ?? []
    } else {
      sourceSkills = []
    }
  }

  let filteredSkillOptions = $derived.by(() => {
    const q = skillSearchQuery.toLowerCase().trim()
    const linkedIds = new Set(sourceSkills.map(s => s.id))
    let options = allSkills.filter(s => !linkedIds.has(s.id))
    if (q) {
      options = options.filter(s => s.name.toLowerCase().includes(q))
    }
    return options.slice(0, 20)
  })

  let canCreateNewSkill = $derived(
    skillSearchQuery.trim().length > 0 &&
    !allSkills.some(s => s.name.toLowerCase() === skillSearchQuery.trim().toLowerCase())
  )

  async function addSkillToSource(skillId: string) {
    if (!selectedId) return
    addingSkill = true
    const res = await fetch(`/api/sources/${selectedId}/skills`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ skill_id: skillId }),
    })
    if (res.ok) {
      const body = await res.json()
      sourceSkills = [...sourceSkills, body.data]
      skillSearchQuery = ''
      showSkillDropdown = false
    }
    addingSkill = false
  }

  async function createAndAddSkill() {
    if (!selectedId || !skillSearchQuery.trim()) return
    addingSkill = true
    const res = await fetch(`/api/sources/${selectedId}/skills`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: skillSearchQuery.trim() }),
    })
    if (res.ok) {
      const body = await res.json()
      sourceSkills = [...sourceSkills, body.data]
      // Also add to allSkills so it appears in future searches
      if (!allSkills.some(s => s.id === body.data.id)) {
        allSkills = [...allSkills, body.data]
      }
      skillSearchQuery = ''
      showSkillDropdown = false
      addToast({ message: `Skill "${body.data.name}" created and linked.`, type: 'success' })
    }
    addingSkill = false
  }

  async function removeSkillFromSource(skillId: string) {
    if (!selectedId) return
    const res = await fetch(`/api/sources/${selectedId}/skills/${skillId}`, { method: 'DELETE' })
    if (res.ok) {
      sourceSkills = sourceSkills.filter(s => s.id !== skillId)
    }
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }
</script>

{#snippet sourceCard(source: Source)}
  <button
    class="source-card"
    class:selected={selectedId === source.id}
    onclick={() => selectSource(source.id)}
  >
    <div class="card-top">
      <span class="card-title">{source.title}</span>
      <StatusBadge status={source.status} />
    </div>
    <div class="card-meta">
      {#if !sourceTypeFilter}
        <span class="type-badge type-{source.source_type}">{source.source_type}</span>
      {/if}
      {#if source.source_type === 'project' && source.project?.is_personal}
        <span class="personal-badge">Personal</span>
      {/if}
      {#if source.source_type === 'project' && (source.project as any)?.open_source}
        <span class="personal-badge" style="background: var(--color-success-subtle); color: var(--color-success-text);">Open Source</span>
      {/if}
      {#if source.source_type === 'role' && source.role?.organization_id}
        <span class="card-org">{getOrgName(source.role.organization_id)}</span>
      {/if}
      {#if source.source_type === 'project' && !source.project?.is_personal && source.project?.organization_id}
        <span class="card-org">{getOrgName(source.project.organization_id)}</span>
      {/if}
      {#if source.source_type === 'education' && source.education?.organization_id}
        <span class="card-org">{getOrgName(source.education.organization_id)}</span>
      {/if}
      {#if source.source_type === 'education' && source.education?.education_type}
        <span class="edu-type-badge">{source.education.education_type}</span>
      {/if}
      {#if source.source_type === 'presentation' && source.presentation?.venue}
        <span class="card-org">{source.presentation.venue}</span>
      {/if}
    </div>
    <span class="card-date">{formatDate(source.updated_at)}</span>
  </button>
{/snippet}

<div class="sources-page">
  <SplitPanel listWidth={340}>
    {#snippet list()}
      <ListPanelHeader title="Sources" onNew={startNew} newLabel="+ New Source" />

      {#if !sourceTypeFilter}
    <div class="filter-tabs">
      {#each SOURCE_TABS as tab}
        <button
          class="filter-tab"
          class:active={activeTab === tab.value}
          onclick={() => activeTab = tab.value}
        >
          {tab.label}
          <span class="tab-count">
            {tab.value === 'all'
              ? sources.length
              : sources.filter(s => s.source_type === tab.value).length}
          </span>
        </button>
      {/each}
    </div>
    {/if}

    {#if activeTab === 'education'}
      <div class="group-bar">
        <label for="edu-group">Group:</label>
        <select id="edu-group" bind:value={eduGroupBy}>
          {#each GROUP_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
    {/if}

    {#if loading}
      <div class="list-loading">
        <LoadingSpinner size="md" message="Loading sources..." />
      </div>
    {:else if filteredSources.length === 0}
      <EmptyState
        title="No sources found"
        description={activeTab === 'all'
          ? 'Create your first source to get started.'
          : `No ${activeTab} sources found.`}
        action={activeTab === 'all' ? 'New Source' : undefined}
        onaction={activeTab === 'all' ? startNew : undefined}
      />
    {:else if groupedEducation}
      <!-- Grouped education list -->
      {#each groupedEducation as group (group.key)}
        <div class="group-section">
          <button
            class="group-header"
            onclick={() => toggleGroupCollapse(group.key)}
          >
            <span class="group-chevron" class:collapsed={collapsedGroups[group.key]}>&#9662;</span>
            <span class="group-label">{group.label}</span>
            <span class="group-count">{group.sources.length}</span>
          </button>
          {#if !collapsedGroups[group.key]}
            <ul class="source-list grouped">
              {#each group.sources as source (source.id)}
                <li>
                  {@render sourceCard(source)}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/each}
    {:else}
      <!-- Flat list -->
      <ul class="source-list">
        {#each filteredSources as source (source.id)}
          <li>
            {@render sourceCard(source)}
          </li>
        {/each}
      </ul>
    {/if}
    {/snippet}

    {#snippet detail()}
    {#if !selectedSource && !editing}
      <EmptyPanel message="Select a source or create a new one." />
    {:else}
      <div class="editor-content">
        <h3 class="editor-heading">{editing ? 'New Source' : 'Edit Source'}</h3>

        <!-- Base fields -->
        <div class="form-group">
          <label for="source-type">Type</label>
          <select id="source-type" bind:value={formSourceType} disabled={!editing}>
            <option value="general">General</option>
            <option value="role">Role</option>
            <option value="project">Project</option>
            <option value="presentation">Presentation</option>
            <option value="education">Education</option>
            <option value="clearance">Clearance</option>
          </select>
        </div>

        <div class="form-group">
          <label for="source-title">Title <span class="required">*</span></label>
          <input
            id="source-title"
            type="text"
            bind:value={formTitle}
            placeholder="e.g. Cloud Migration Project"
          />
        </div>

        <div class="form-group">
          <label for="source-description">Description <span class="required">*</span></label>
          <textarea
            id="source-description"
            bind:value={formDescription}
            rows="6"
            placeholder="Describe what you did, the context, technologies used..."
          ></textarea>
        </div>

        <!-- Role-specific fields -->
        {#if formSourceType === 'role'}
          <div class="form-group">
            <label for="role-org">Organization</label>
            <select id="role-org" bind:value={formOrgId}>
              <option value={null}>None</option>
              {#each roleFilteredOrgs as org}
                <option value={org.id}>{org.name}</option>
              {/each}
            </select>
          </div>
          <div class="form-row">
            <div class="form-group">
              <label for="role-start">Start Date</label>
              <input id="role-start" type="date" bind:value={formStartDate} />
            </div>
            <div class="form-group">
              <label for="role-end">End Date</label>
              <input id="role-end" type="date" bind:value={formEndDate} />
            </div>
          </div>
          <div class="form-group">
            <label>
              <input type="checkbox" bind:checked={formIsCurrent} /> Currently employed
            </label>
          </div>
          <div class="form-group">
            <label for="role-arrangement">Work Arrangement</label>
            <input id="role-arrangement" type="text" bind:value={formWorkArrangement}
                   placeholder="e.g. remote, hybrid, on-site" />
          </div>
        {/if}

        <!-- Education-specific fields -->
        {#if formSourceType === 'education'}
          <div class="form-group">
            <label for="edu-type">Education Type</label>
            <select id="edu-type" value={formEducationType} onchange={(e) => handleEducationTypeChange((e.target as HTMLSelectElement).value)}>
              <option value="degree">Degree</option>
              <option value="course">Course</option>
              <option value="self_taught">Self-Taught</option>
            </select>
          </div>

          {#if formEducationType === 'degree'}
            <!-- Degree fields -->
            <div class="form-row">
              <div class="form-group">
                <label for="edu-degree-level">Degree Level <span class="required">*</span></label>
                <select id="edu-degree-level" bind:value={formDegreeLevel}>
                  <option value="" disabled>-- Select Degree Level --</option>
                  <option value="associate">Associate</option>
                  <option value="bachelors">Bachelor's</option>
                  <option value="masters">Master's</option>
                  <option value="doctoral">Doctoral</option>
                  <option value="graduate_certificate">Graduate Certificate</option>
                </select>
              </div>
              <div class="form-group">
                <label for="edu-degree-type">Degree Type <span class="required">*</span></label>
                <input id="edu-degree-type" type="text" bind:value={formDegreeType}
                       placeholder="e.g. BS, MS, PhD, MBA" />
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="edu-org">Institution</label>
                <div class="org-select-row">
                  <OrgCombobox
                    id="edu-org"
                    organizations={eduFilteredOrgs}
                    bind:value={formEduOrgId}
                    placeholder="Search institutions..."
                    oncreate={openOrgModal}
                  />
                </div>
              </div>
              <div class="form-group">
                <label for="edu-campus">Campus</label>
                <div class="org-select-row">
                  <select id="edu-campus" bind:value={formCampusId} disabled={!formEduOrgId}>
                    <option value={null}>{formEduOrgId ? '-- Select --' : 'Select org first'}</option>
                    {#each campuses as campus}
                      <option value={campus.id}>{campus.name}{campus.city ? ` (${campus.city}${campus.state ? `, ${campus.state}` : ''})` : ''}</option>
                    {/each}
                  </select>
                  <button class="btn-new-sm" onclick={openCampusModal} type="button" disabled={!formEduOrgId}>+</button>
                </div>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="edu-field">Field (Major)</label>
                <input id="edu-field" type="text" bind:value={formField} />
              </div>
              <div class="form-group">
                <label for="edu-gpa">GPA</label>
                <input id="edu-gpa" type="text" bind:value={formGpa}
                       placeholder="e.g. 3.8/4.0" />
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="edu-start">Start Date</label>
                <input id="edu-start" type="date" bind:value={formStartDate} />
              </div>
              <div class="form-group">
                <label for="edu-end">End Date</label>
                <input id="edu-end" type="date" bind:value={formEndDate} />
              </div>
            </div>
            <div class="form-group">
              <label>
                <input type="checkbox" bind:checked={formIsInProgress} /> In progress
              </label>
            </div>
            <div class="form-group">
              <label for="edu-description">Description</label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="3"
                        placeholder="Additional notes about this degree..."></textarea>
            </div>

          {:else if formEducationType === 'course'}
            <!-- Course fields -->
            <div class="form-row">
              <div class="form-group">
                <label for="edu-org">Institution (Provider)</label>
                <div class="org-select-row">
                  <OrgCombobox
                    id="edu-org"
                    organizations={eduFilteredOrgs}
                    bind:value={formEduOrgId}
                    placeholder="Search providers..."
                    oncreate={openOrgModal}
                  />
                </div>
              </div>
              <div class="form-group">
                <label for="edu-campus">Campus / Location</label>
                <div class="org-select-row">
                  <select id="edu-campus" bind:value={formCampusId} disabled={!formEduOrgId}>
                    <option value={null}>{formEduOrgId ? '-- Select --' : 'Select org first'}</option>
                    {#each campuses as campus}
                      <option value={campus.id}>{campus.name}{campus.city ? ` (${campus.city}${campus.state ? `, ${campus.state}` : ''})` : ''}</option>
                    {/each}
                  </select>
                  <button class="btn-new-sm" onclick={openCampusModal} type="button" disabled={!formEduOrgId}>+</button>
                </div>
              </div>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="edu-start">Start Date</label>
                <input id="edu-start" type="date" bind:value={formStartDate} />
              </div>
              <div class="form-group">
                <label for="edu-end">End Date</label>
                <input id="edu-end" type="date" bind:value={formEndDate} />
              </div>
            </div>
            <div class="form-group">
              <label for="edu-url">URL</label>
              <input id="edu-url" type="url" bind:value={formUrl} />
            </div>
            <div class="form-group">
              <label for="edu-description">Description</label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="3"
                        placeholder="What you learned, key takeaways..."></textarea>
            </div>

          {:else if formEducationType === 'self_taught'}
            <!-- Self-taught fields -->
            <div class="form-group">
              <label for="edu-description">Description <span class="required">*</span></label>
              <textarea id="edu-description" bind:value={formEduDescription} rows="6"
                        placeholder="Describe what you learned, resources used, projects built..."></textarea>
            </div>
            <div class="form-group">
              <label for="edu-url">URL</label>
              <input id="edu-url" type="url" bind:value={formUrl} />
            </div>
          {/if}
        {/if}

        <!-- Project-specific fields -->
        {#if formSourceType === 'project'}
          <div class="form-group" style="display: flex; gap: 1.5rem;">
            <label>
              <input type="checkbox" bind:checked={formIsPersonal} /> Personal project
            </label>
            <label>
              <input type="checkbox" bind:checked={formOpenSource} /> Open source
            </label>
          </div>
          {#if !formIsPersonal}
            <div class="form-group">
              <label for="proj-org">Organization</label>
              <select id="proj-org" bind:value={formOrgId}>
                <option value={null}>None</option>
                {#each roleFilteredOrgs as org}
                  <option value={org.id}>{org.name}</option>
                {/each}
              </select>
            </div>
          {/if}
          <div class="form-group">
            <label for="proj-url">URL</label>
            <input id="proj-url" type="url" bind:value={formProjectUrl} />
          </div>
          <div class="form-row">
            <div class="form-group">
              <label for="proj-start">Start Date</label>
              <input id="proj-start" type="date" bind:value={formStartDate} />
            </div>
            <div class="form-group">
              <label for="proj-end">End Date</label>
              <input id="proj-end" type="date" bind:value={formEndDate} />
            </div>
          </div>
        {/if}

        <!-- Presentation-specific fields -->
        {#if formSourceType === 'presentation'}
          <div class="form-row">
            <div class="form-group">
              <label for="pres-venue">Venue / Event</label>
              <input id="pres-venue" type="text" bind:value={formVenue}
                     placeholder="e.g. BSides DC 2024, AWS re:Inforce" />
            </div>
            <div class="form-group">
              <label for="pres-type">Type</label>
              <select id="pres-type" bind:value={formPresentationType}>
                <option value="conference_talk">Conference Talk</option>
                <option value="workshop">Workshop</option>
                <option value="poster">Poster</option>
                <option value="webinar">Webinar</option>
                <option value="lightning_talk">Lightning Talk</option>
                <option value="panel">Panel</option>
                <option value="internal">Internal</option>
              </select>
            </div>
          </div>
          <div class="form-row">
            <div class="form-group">
              <label for="pres-start">Date</label>
              <input id="pres-start" type="date" bind:value={formStartDate} />
            </div>
            <div class="form-group">
              <label for="pres-url">URL (slides/recording)</label>
              <input id="pres-url" type="url" bind:value={formPresentationUrl}
                     placeholder="https://..." />
            </div>
          </div>
          <div class="form-group">
            <label for="pres-coauthors">Co-authors</label>
            <input id="pres-coauthors" type="text" bind:value={formCoauthors}
                   placeholder="e.g. Jane Smith, Bob Lee" />
          </div>
        {/if}

        <!-- Clearance-specific fields -->
        {#if formSourceType === 'clearance'}
          <div class="form-row">
            <div class="form-group">
              <label for="clearance-level">Level <span class="required">*</span></label>
              <select id="clearance-level" bind:value={formLevel}>
                {#each CLEARANCE_LEVELS as level}
                  <option value={level}>{CLEARANCE_LEVEL_LABELS[level]}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label for="clearance-status">Status</label>
              <select id="clearance-status" bind:value={formClearanceStatus}>
                {#each CLEARANCE_STATUSES as status}
                  <option value={status}>{status.charAt(0).toUpperCase() + status.slice(1)}</option>
                {/each}
              </select>
            </div>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="clearance-polygraph">Polygraph</label>
              <select id="clearance-polygraph" bind:value={formPolygraph}>
                <option value="">-- None --</option>
                {#each CLEARANCE_POLYGRAPHS as poly}
                  <option value={poly}>{CLEARANCE_POLYGRAPH_LABELS[poly]}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label for="clearance-type">Type</label>
              <select id="clearance-type" bind:value={formClearanceType}>
                {#each CLEARANCE_TYPES as ctype}
                  <option value={ctype}>{ctype.charAt(0).toUpperCase() + ctype.slice(1)}</option>
                {/each}
              </select>
            </div>
          </div>

          <div class="form-group">
            <label>Access Programs</label>
            <div class="checkbox-group">
              {#each CLEARANCE_ACCESS_PROGRAMS as prog}
                <label class="checkbox-label">
                  <input
                    type="checkbox"
                    checked={formAccessPrograms.includes(prog)}
                    onchange={(e) => {
                      if (e.currentTarget.checked) {
                        formAccessPrograms = [...formAccessPrograms, prog]
                      } else {
                        formAccessPrograms = formAccessPrograms.filter(p => p !== prog)
                      }
                    }}
                  />
                  {CLEARANCE_ACCESS_PROGRAM_LABELS[prog]}
                </label>
              {/each}
            </div>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input
                type="checkbox"
                bind:checked={formContinuousInvestigation}
              />
              Continuous Investigation (CE/CV)
            </label>
          </div>
        {/if}

        <!-- Notes (all types) -->
        <div class="form-group">
          <label for="source-notes">Notes</label>
          <textarea id="source-notes" bind:value={formNotes} rows="3"
                    placeholder="Internal notes about this source..."></textarea>
        </div>

        <!-- Skills (all types, only for existing sources) -->
        {#if selectedId && !editing}
          <div class="skills-section">
            <label>Skills</label>
            {#if sourceSkills.length > 0}
              <div class="skill-pills">
                {#each sourceSkills as skill (skill.id)}
                  <span class="skill-pill">
                    {skill.name}
                    <button class="skill-remove" onclick={() => removeSkillFromSource(skill.id)} title="Remove">×</button>
                  </span>
                {/each}
              </div>
            {/if}
            <div class="skill-add-row">
              <input
                type="text"
                bind:value={skillSearchQuery}
                placeholder="Search or create skill..."
                onfocus={() => showSkillDropdown = true}
                onblur={() => setTimeout(() => showSkillDropdown = false, 200)}
              />
              {#if showSkillDropdown && (filteredSkillOptions.length > 0 || canCreateNewSkill)}
                <ul class="skill-dropdown">
                  {#each filteredSkillOptions as skill (skill.id)}
                    <li>
                      <button onmousedown={() => addSkillToSource(skill.id)}>
                        {skill.name}
                        {#if skill.category}<span class="skill-cat">{skill.category}</span>{/if}
                      </button>
                    </li>
                  {/each}
                  {#if canCreateNewSkill}
                    <li class="create-new">
                      <button onmousedown={createAndAddSkill}>
                        + Create "{skillSearchQuery.trim()}"
                      </button>
                    </li>
                  {/if}
                </ul>
              {/if}
            </div>
          </div>
        {/if}

        <div class="editor-actions">
          <button
            class="btn btn-save"
            onclick={saveSource}
            disabled={saving}
          >
            {#if saving}
              <LoadingSpinner size="sm" />
            {:else}
              {editing ? 'Create' : 'Save'}
            {/if}
          </button>

          {#if !editing && selectedSource}
            {#if selectedSource.status === 'draft' || selectedSource.status === 'approved'}
              {#if false}
                <!-- Derive button hidden: server-side derivation disabled, use MCP tools -->
                <button
                  class="btn btn-derive"
                  onclick={deriveBullets}
                  disabled={deriving}
                >
                  {#if deriving}
                    <LoadingSpinner size="sm" />
                    <span>Deriving...</span>
                  {:else}
                    Derive Bullets
                  {/if}
                </button>
              {/if}
            {/if}

            <button
              class="btn btn-delete"
              onclick={() => confirmDeleteOpen = true}
            >
              Delete
            </button>
          {/if}
        </div>
      </div>
    {/if}
    {/snippet}
  </SplitPanel>
</div>

<ConfirmDialog
  open={confirmDeleteOpen}
  title="Delete Source"
  message="Are you sure you want to delete this source? This action cannot be undone."
  confirmLabel="Delete"
  onconfirm={deleteSource}
  oncancel={() => confirmDeleteOpen = false}
  destructive
/>

<!-- Quick-create organization modal -->
{#if showOrgModal}
  <div class="modal-overlay" onclick={() => showOrgModal = false} role="presentation">
    <div class="org-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="org-modal-title">
      <h3 id="org-modal-title">New Organization</h3>
      <div class="form-group">
        <label for="new-org-name">Name *</label>
        <input id="new-org-name" type="text" bind:value={newOrgName} placeholder="e.g. Georgia Tech, CompTIA, Coursera" />
      </div>
      <div class="form-group">
        <label for="new-org-type">Primary Type</label>
        <select id="new-org-type" bind:value={newOrgType}>
          <option value="education">Education</option>
          <option value="company">Company</option>
          <option value="nonprofit">Nonprofit</option>
          <option value="government">Government</option>
          <option value="military">Military</option>
          <option value="other">Other</option>
        </select>
      </div>
      <div class="form-group">
        <label>Tags</label>
        <div class="tag-checkboxes">
          {#each ['company', 'vendor', 'platform', 'university', 'school', 'nonprofit', 'government', 'military', 'conference', 'volunteer', 'freelance'] as tag}
            <label class="tag-checkbox">
              <input type="checkbox" checked={newOrgTags.includes(tag)}
                onchange={(e) => {
                  const target = e.target as HTMLInputElement
                  if (target.checked) {
                    newOrgTags = [...newOrgTags, tag]
                  } else {
                    newOrgTags = newOrgTags.filter(t => t !== tag)
                  }
                }}
              />
              {tag}
            </label>
          {/each}
        </div>
      </div>
      <div class="form-group">
        <label for="new-org-website">Website</label>
        <input id="new-org-website" type="url" bind:value={newOrgWebsite} placeholder="https://..." />
      </div>
      <div class="modal-actions">
        <button class="btn btn-ghost" onclick={() => showOrgModal = false} disabled={creatingOrg}>Cancel</button>
        <button class="btn btn-primary" onclick={createOrgAndSelect} disabled={creatingOrg || !newOrgName.trim()}>
          {creatingOrg ? 'Creating...' : 'Create & Select'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Quick-create campus modal -->
{#if showCampusModal}
  <div class="modal-overlay" onclick={() => showCampusModal = false} role="presentation">
    <div class="org-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="campus-modal-title">
      <h3 id="campus-modal-title">New Campus</h3>
      <div class="form-group">
        <label for="new-campus-name">Name *</label>
        <input id="new-campus-name" type="text" bind:value={newCampusName} placeholder="e.g. Main Campus, Online, Ashburn" />
      </div>
      <div class="form-group">
        <label for="new-campus-modality">Modality</label>
        <select id="new-campus-modality" bind:value={newCampusModality}>
          <option value="in_person">In Person</option>
          <option value="remote">Remote / Online</option>
          <option value="hybrid">Hybrid</option>
        </select>
      </div>
      <div class="form-row">
        <div class="form-group">
          <label for="new-campus-city">City</label>
          <input id="new-campus-city" type="text" bind:value={newCampusCity} placeholder="e.g. Washington" />
        </div>
        <div class="form-group">
          <label for="new-campus-state">State</label>
          <input id="new-campus-state" type="text" bind:value={newCampusState} placeholder="e.g. DC" />
        </div>
      </div>
      <div class="modal-actions">
        <button class="btn btn-ghost" onclick={() => showCampusModal = false} disabled={creatingCampus}>Cancel</button>
        <button class="btn btn-primary" onclick={createCampusAndSelect} disabled={creatingCampus || !newCampusName.trim()}>
          {creatingCampus ? 'Creating...' : 'Create & Select'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .sources-page {
    display: flex;
    gap: 0;
    height: 100%;
  }

  /* ---- Group bar ---- */
  .group-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    font-size: var(--text-sm);
    flex-shrink: 0;
  }

  .group-bar label {
    color: var(--text-muted);
    font-weight: var(--font-medium);
  }

  .group-bar select {
    font-size: var(--text-sm);
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--text-primary);
  }

  /* ---- Group sections ---- */
  .group-section {
    border-bottom: 1px solid var(--color-surface-sunken);
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.6rem 1rem;
    background: var(--color-surface-raised);
    border: none;
    cursor: pointer;
    font-size: var(--text-sm);
    font-weight: var(--font-semibold);
    color: var(--text-secondary);
    text-align: left;
  }

  .group-header:hover {
    background: var(--color-surface-sunken);
  }

  .group-chevron {
    font-size: 0.7rem;
    transition: transform 0.15s;
    display: inline-block;
  }

  .group-chevron.collapsed {
    transform: rotate(-90deg);
  }

  .group-label {
    flex: 1;
  }

  .group-count {
    color: var(--text-faint);
    font-weight: var(--font-normal);
    font-size: 0.75rem;
  }

  .source-list.grouped {
    padding-left: 0;
  }

  /* ---- Modal overlay ---- */
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    background: var(--color-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* ---- Org select row ---- */
  .org-select-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  .org-select-row select {
    flex: 1;
    min-width: 0;
  }

  .btn-new-sm {
    padding: 0.3rem 0.6rem;
    background: var(--color-primary);
    color: var(--text-inverse);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: var(--font-semibold);
    flex-shrink: 0;
    line-height: 1;
  }

  .btn-new-sm:hover { background: var(--color-primary-hover); }

  /* ---- Org modal ---- */
  .org-modal {
    background: var(--color-surface);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    width: 400px;
    max-width: 90vw;
    box-shadow: var(--shadow-lg);
  }

  .org-modal h3 {
    margin: 0 0 1rem;
    font-size: var(--text-xl);
    color: var(--text-primary);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }

  .tag-checkboxes {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 0.8rem;
    font-size: var(--text-sm);
  }

  .tag-checkbox {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
    color: var(--text-secondary);
  }

  .tag-checkbox input[type='checkbox'] {
    margin: 0;
  }

  /* ---- Source Skills ---- */
  .skills-section { margin-bottom: 1.25rem; }
  .skills-section > label { display: block; font-size: var(--text-sm); font-weight: var(--font-medium); color: var(--text-secondary); margin-bottom: 0.35rem; }
  .skill-pills { display: flex; flex-wrap: wrap; gap: 0.3rem; margin-bottom: 0.5rem; }
  .skill-pill { display: inline-flex; align-items: center; gap: 0.2rem; padding: 0.15rem 0.5rem; background: var(--color-info-subtle); color: var(--color-info-text); border-radius: 12px; font-size: 0.75rem; font-weight: var(--font-medium); }
  .skill-remove { background: none; border: none; color: var(--color-info); cursor: pointer; font-size: 0.85rem; padding: 0; line-height: 1; }
  .skill-remove:hover { color: var(--color-danger); }
  .skill-add-row { position: relative; }
  .skill-add-row input { width: 100%; padding: 0.4rem 0.6rem; border: 1px solid var(--color-border-strong); border-radius: var(--radius-md); font-size: var(--text-sm); color: var(--text-primary); background: var(--color-surface); }
  .skill-add-row input:focus { outline: none; border-color: var(--color-border-focus); }
  .skill-dropdown { position: absolute; top: 100%; left: 0; right: 0; background: var(--color-surface); border: 1px solid var(--color-border-strong); border-radius: var(--radius-md); box-shadow: var(--shadow-md); list-style: none; padding: 0.25rem 0; margin: 0.25rem 0 0; max-height: 200px; overflow-y: auto; z-index: var(--z-dropdown); }
  .skill-dropdown li button { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 0.4rem 0.75rem; background: none; border: none; cursor: pointer; font-size: var(--text-sm); color: var(--text-secondary); text-align: left; }
  .skill-dropdown li button:hover { background: var(--color-surface-sunken); }
  .skill-dropdown .create-new button { color: var(--color-primary); font-weight: var(--font-medium); }
  .skill-cat { font-size: 0.65rem; color: var(--text-faint); padding: 0.1rem 0.3rem; background: var(--color-surface-sunken); border-radius: var(--radius-sm); }

  .edu-type-badge {
    font-size: var(--text-xs);
    padding: 0.1rem 0.4rem;
    border-radius: var(--radius-sm);
    background: var(--color-tag-bg);
    color: var(--color-tag-text);
    text-transform: capitalize;
  }

  /* ---- Left panel ---- */
  .filter-tabs {
    display: flex;
    border-bottom: 1px solid var(--color-border);
    padding: 0 0.25rem;
    overflow-x: auto;
  }

  .filter-tab {
    flex: 1;
    padding: 0.5rem 0.35rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: 0.72rem;
    font-weight: var(--font-medium);
    color: var(--text-muted);
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
    text-align: center;
    white-space: nowrap;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
  }

  .filter-tab:hover {
    color: var(--text-secondary);
  }

  .filter-tab.active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
  }

  .tab-count {
    font-size: 0.65rem;
    color: var(--text-faint);
    font-weight: var(--font-normal);
  }

  .filter-tab.active .tab-count {
    color: var(--color-primary);
  }

  .list-loading {
    display: flex;
    justify-content: center;
    padding: 3rem 1rem;
  }

  .source-list {
    list-style: none;
    overflow-y: auto;
    flex: 1;
  }

  .source-card {
    display: block;
    width: 100%;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-surface-sunken);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s;
  }

  .source-card:hover {
    background: var(--color-surface-raised);
  }

  .source-card.selected {
    background: var(--color-primary-subtle);
    border-left: 3px solid var(--color-primary);
    padding-left: calc(1rem - 3px);
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .card-title {
    font-size: var(--text-base);
    font-weight: var(--font-medium);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin-bottom: 0.25rem;
  }

  .card-org {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .type-badge {
    display: inline-block;
    padding: 0.1em 0.4em;
    border-radius: var(--radius-sm);
    font-size: 0.65rem;
    font-weight: var(--font-semibold);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .type-role { background: var(--color-info-subtle); color: var(--color-info-text); }
  .type-project { background: var(--color-warning-bg); color: var(--color-warning-text); }
  .type-education { background: var(--color-success-subtle); color: var(--color-success-text); }
  .type-clearance { background: #fce7f3; color: #9d174d; }
  .type-general { background: var(--color-surface-sunken); color: var(--text-muted); }

  .card-date {
    font-size: 0.75rem;
    color: var(--text-faint);
  }

  /* ---- Right panel ---- */
  .editor-content {
    max-width: 640px;
    padding: 2rem;
  }

  .editor-heading {
    font-size: var(--text-xl);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    margin-bottom: var(--space-6);
  }

  .form-group {
    margin-bottom: 1.25rem;
  }

  .form-group label {
    display: block;
    font-size: var(--text-sm);
    font-weight: var(--font-medium);
    color: var(--text-secondary);
    margin-bottom: 0.35rem;
  }

  .required {
    color: var(--color-danger);
  }

  .form-group input[type='text'],
  .form-group input[type='date'],
  .form-group input[type='url'],
  .form-group input[type='number'],
  .form-group textarea,
  .form-group select {
    width: 100%;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    color: var(--text-primary);
    background: var(--color-surface);
    transition: border-color 0.15s;
    font-family: inherit;
  }

  .form-group input:focus,
  .form-group textarea:focus,
  .form-group select:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 2px var(--color-primary-subtle);
  }

  .form-group select:disabled {
    background: var(--color-surface-sunken);
    color: var(--text-faint);
    cursor: not-allowed;
  }

  .form-group textarea {
    resize: vertical;
    min-height: 100px;
    line-height: var(--leading-normal);
  }

  .form-row {
    display: flex;
    gap: 1rem;
  }

  .form-row .form-group {
    flex: 1;
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-6);
    padding-top: var(--space-5);
    border-top: 1px solid var(--color-border);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 1.1rem;
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.85rem;
    font-weight: var(--font-medium);
    cursor: pointer;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-save {
    background: var(--color-primary);
    color: var(--text-inverse);
  }

  .btn-save:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  .btn-derive {
    background: var(--color-primary);
    color: var(--text-inverse);
  }

  .btn-derive:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  .btn-delete {
    background: var(--color-danger-subtle);
    color: var(--color-danger-text);
    margin-left: auto;
  }

  .btn-delete:hover {
    background: var(--color-danger-subtle);
  }

  .personal-badge {
    display: inline-block;
    padding: 0.1em 0.4em;
    background: #ede9fe;
    color: #6d28d9;
    border-radius: var(--radius-sm);
    font-size: 0.65rem;
    font-weight: var(--font-semibold);
    text-transform: uppercase;
  }

  .checkbox-group {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
</style>
