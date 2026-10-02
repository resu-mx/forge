# Phase 13: CLI Updates + v1 Data Import

**Goal:** Update the `forge` CLI for the evolved schema (organizations, notes, skills commands, updated source commands) and implement the `forge import v1` command to migrate all v1 data into Forge.

**Non-Goals:** No UI. No direct database access from CLI (except the import command, which may use direct DB for performance). No interactive review changes (existing `forge review` still works).

**Depends on:** Phase 12 (SDK — CLI is an SDK consumer)
**Blocks:** Phase 14 (UI needs data to display)

---

## Task 13.1: Update CLI Source Commands

**Goal:** Update source commands for polymorphic source_type + extension data.

**File:** `packages/cli/src/commands/source.ts`

**Changes:**

1. `forge source add` accepts `--type` flag (`role`, `project`, `education`, `clearance`, `general`; default: `general`)
2. `forge source add` accepts type-specific flags:
   - For `role`: `--org`, `--start-date`, `--end-date`, `--current`, `--arrangement`
   - For `education`: `--edu-type` (degree/certificate/course/self_taught), `--institution`, `--field`, `--credential-id`
   - For `clearance`: `--level`, `--polygraph`, `--clearance-status`
   - For `project`: `--org`, `--personal`, `--url`, `--start-date`, `--end-date`
3. `forge source list` shows `source_type` column, supports `--type` filter (replaces `--employer-id` and `--project-id`)
4. `forge source show` displays extension data based on type

**Implementation:**

```typescript
import { defineCommand } from 'citty'
import { forge, exitOnNetworkError } from '../client'

const add = defineCommand({
  meta: { name: 'add', description: 'Add a new source' },
  args: {
    title: {
      type: 'string',
      description: 'Source title',
      required: true,
    },
    description: {
      type: 'string',
      description: 'Source description',
      required: true,
    },
    type: {
      type: 'string',
      description: 'Source type (role, project, education, clearance, general)',
      default: 'general',
    },
    // Role-specific
    org: {
      type: 'string',
      description: 'Organization ID (for role/project)',
    },
    'start-date': {
      type: 'string',
      description: 'Start date (YYYY-MM-DD)',
    },
    'end-date': {
      type: 'string',
      description: 'End date (YYYY-MM-DD)',
    },
    current: {
      type: 'boolean',
      description: 'Is current role/position',
    },
    arrangement: {
      type: 'string',
      description: 'Work arrangement (remote, hybrid, onsite)',
    },
    // Education-specific
    'edu-type': {
      type: 'string',
      description: 'Education type (degree, certificate, course, self_taught)',
    },
    institution: {
      type: 'string',
      description: 'Institution name',
    },
    field: {
      type: 'string',
      description: 'Field of study',
    },
    'credential-id': {
      type: 'string',
      description: 'Credential ID',
    },
    // Clearance-specific
    level: {
      type: 'string',
      description: 'Clearance level',
    },
    polygraph: {
      type: 'string',
      description: 'Polygraph type (None, CI, Full Scope)',
    },
    'clearance-status': {
      type: 'string',
      description: 'Clearance status (active, inactive, expired)',
    },
    // Project-specific
    personal: {
      type: 'boolean',
      description: 'Is personal project',
    },
    url: {
      type: 'string',
      description: 'Project URL',
    },
  },
  async run({ args }) {
    const input: Record<string, unknown> = {
      title: args.title,
      description: args.description,
      source_type: args.type,
    }

    // Build extension object based on type
    if (args.type === 'role') {
      const role: Record<string, unknown> = {}
      if (args.org) role.organization_id = args.org
      if (args['start-date']) role.start_date = args['start-date']
      if (args['end-date']) role.end_date = args['end-date']
      if (args.current !== undefined) role.is_current = args.current
      if (args.arrangement) role.work_arrangement = args.arrangement
      input.role = role
    } else if (args.type === 'education') {
      const education: Record<string, unknown> = {}
      if (args['edu-type']) education.education_type = args['edu-type']
      if (args.institution) education.institution = args.institution
      if (args.field) education.field = args.field
      if (args['credential-id']) education.credential_id = args['credential-id']
      if (args['start-date']) education.start_date = args['start-date']
      if (args['end-date']) education.end_date = args['end-date']
      input.education = education
    } else if (args.type === 'clearance') {
      const clearance: Record<string, unknown> = {}
      if (args.level) clearance.level = args.level
      if (args.polygraph) clearance.polygraph = args.polygraph
      if (args['clearance-status']) clearance.status = args['clearance-status']
      input.clearance = clearance
    } else if (args.type === 'project') {
      const project: Record<string, unknown> = {}
      if (args.org) project.organization_id = args.org
      if (args.personal !== undefined) project.is_personal = args.personal
      if (args.url) project.url = args.url
      if (args['start-date']) project.start_date = args['start-date']
      if (args['end-date']) project.end_date = args['end-date']
      input.project = project
    }

    const result = await forge.sources.create(input as any)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const s = result.data
    console.log(`Created source: ${s.title}`)
    console.log(`  ID:   ${s.id}`)
    console.log(`  Type: ${s.source_type}`)
    console.log(`  Status: ${s.status}`)
  },
})

const list = defineCommand({
  meta: { name: 'list', description: 'List all sources' },
  args: {
    status: {
      type: 'string',
      description: 'Filter by status (draft, approved, deriving)',
    },
    type: {
      type: 'string',
      description: 'Filter by source type (role, project, education, clearance, general)',
    },
    offset: {
      type: 'string',
      description: 'Pagination offset',
    },
    limit: {
      type: 'string',
      description: 'Pagination limit',
    },
  },
  async run({ args }) {
    const filter: Record<string, string | number> = {}
    if (args.status) filter.status = args.status
    if (args.type) filter.source_type = args.type
    if (args.offset) filter.offset = Number(args.offset)
    if (args.limit) filter.limit = Number(args.limit)

    const result = await forge.sources.list(
      Object.keys(filter).length > 0 ? (filter as any) : undefined,
    )
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')

    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    if (result.data.length === 0) {
      console.log('No sources found.')
      return
    }

    // Table output — now includes Type column
    const idW = 10
    const titleW = 28
    const typeW = 11
    const statusW = 11
    const createdW = 12

    const header = [
      'ID'.padEnd(idW),
      'Title'.padEnd(titleW),
      'Type'.padEnd(typeW),
      'Status'.padEnd(statusW),
      'Created'.padEnd(createdW),
    ].join('  ')

    const divider = [
      '\u2500'.repeat(idW),
      '\u2500'.repeat(titleW),
      '\u2500'.repeat(typeW),
      '\u2500'.repeat(statusW),
      '\u2500'.repeat(createdW),
    ].join('  ')

    console.log(header)
    console.log(divider)

    for (const s of result.data) {
      const id = s.id.length > 8 ? s.id.slice(0, 6) + '..' : s.id
      const title =
        s.title.length > titleW ? s.title.slice(0, titleW - 1) + '\u2026' : s.title
      const created = s.created_at.slice(0, 10)

      console.log(
        [
          id.padEnd(idW),
          title.padEnd(titleW),
          s.source_type.padEnd(typeW),
          s.status.padEnd(statusW),
          created.padEnd(createdW),
        ].join('  '),
      )
    }

    console.log(
      `\n${result.data.length} of ${result.pagination.total} sources`,
    )
  },
})

const show = defineCommand({
  meta: { name: 'show', description: 'Show details of a source' },
  args: {
    id: { type: 'positional', description: 'Source ID', required: true },
  },
  async run({ args }) {
    const result = await forge.sources.get(args.id)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const s = result.data

    const useJson = process.argv.includes('--json')
    if (useJson) {
      console.log(JSON.stringify(s, null, 2))
      return
    }

    console.log(`Source: ${s.title}`)
    console.log(`  ID:          ${s.id}`)
    console.log(`  Type:        ${s.source_type}`)
    console.log(`  Status:      ${s.status}`)
    console.log(`  Description: ${s.description}`)
    console.log(`  Created:     ${s.created_at}`)
    console.log(`  Updated:     ${s.updated_at}`)

    // Show extension data based on type
    if (s.source_type === 'role' && s.role) {
      console.log(`  Organization:    ${s.role.organization_id ?? '(none)'}`)
      console.log(`  Start Date:      ${s.role.start_date ?? '(none)'}`)
      console.log(`  End Date:        ${s.role.end_date ?? '(none)'}`)
      console.log(`  Current:         ${s.role.is_current ? 'Yes' : 'No'}`)
      console.log(`  Arrangement:     ${s.role.work_arrangement ?? '(none)'}`)
    } else if (s.source_type === 'education' && s.education) {
      console.log(`  Education Type:  ${s.education.education_type}`)
      console.log(`  Institution:     ${s.education.institution ?? '(none)'}`)
      console.log(`  Field:           ${s.education.field ?? '(none)'}`)
      console.log(`  Credential ID:   ${s.education.credential_id ?? '(none)'}`)
    } else if (s.source_type === 'clearance' && s.clearance) {
      console.log(`  Level:           ${s.clearance.level}`)
      console.log(`  Polygraph:       ${s.clearance.polygraph ?? '(none)'}`)
      console.log(`  Status:          ${s.clearance.status ?? '(none)'}`)
    } else if (s.source_type === 'project' && s.project) {
      console.log(`  Organization:    ${s.project.organization_id ?? '(none)'}`)
      console.log(`  Personal:        ${s.project.is_personal ? 'Yes' : 'No'}`)
      console.log(`  URL:             ${s.project.url ?? '(none)'}`)
    }
  },
})

// edit and delete commands updated similarly (remove employer-id/project-id flags,
// add type-aware extension field editing)

export const sourceCommand = defineCommand({
  meta: { name: 'source', description: 'Manage experience sources' },
  subCommands: {
    add,
    list,
    show,
    edit,      // updated: remove employer-id/project-id, add extension fields
    delete: del,
    'derive-bullets': deriveBullets,  // unchanged
  },
})
```

**Acceptance Criteria:**
- [ ] `forge source add --title "Engineer" --description "..." --type role --org <id> --start-date 2023-01-01 --current` creates a role source with extension data
- [ ] `forge source add --title "Note" --description "..."` creates a general source (default type)
- [ ] `forge source list` shows Type column
- [ ] `forge source list --type role` filters to role sources only
- [ ] `forge source list` does NOT accept `--employer-id` or `--project-id` (removed)
- [ ] `forge source show <id>` displays extension data based on source type
- [ ] `forge source edit <id> --title "New Title"` still works for base fields
- [ ] `--json` output includes `source_type` and extension objects

---

## Task 13.2: CLI Organization Commands

**Goal:** Full CRUD for organizations via CLI.

**File:** `packages/cli/src/commands/organization.ts` (NEW)

**Implementation:**

```typescript
import { defineCommand } from 'citty'
import { forge, exitOnNetworkError } from '../client'

const add = defineCommand({
  meta: { name: 'add', description: 'Add a new organization' },
  args: {
    name: {
      type: 'string',
      description: 'Organization name',
      required: true,
    },
    type: {
      type: 'string',
      description: 'Organization type (company, nonprofit, government, military, education, volunteer, freelance, other)',
      default: 'company',
    },
    industry: {
      type: 'string',
      description: 'Industry',
    },
    size: {
      type: 'string',
      description: 'Size (startup, small, medium, large, enterprise)',
    },
    worked: {
      type: 'boolean',
      description: 'Did you work here?',
      default: false,
    },
    'employment-type': {
      type: 'string',
      description: 'Employment type (civilian, contractor, military_active, military_reserve, volunteer, intern)',
    },
    location: {
      type: 'string',
      description: 'Location',
    },
    website: {
      type: 'string',
      description: 'Website URL',
    },
  },
  async run({ args }) {
    const input: Record<string, unknown> = {
      name: args.name,
      org_type: args.type,
      worked: args.worked,
    }
    if (args.industry) input.industry = args.industry
    if (args.size) input.size = args.size
    if (args['employment-type']) input.employment_type = args['employment-type']
    if (args.location) input.location = args.location
    if (args.website) input.website = args.website

    const result = await forge.organizations.create(input as any)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const o = result.data
    console.log(`Created organization: ${o.name}`)
    console.log(`  ID:   ${o.id}`)
    console.log(`  Type: ${o.org_type}`)
  },
})

const list = defineCommand({
  meta: { name: 'list', description: 'List organizations' },
  args: {
    type: {
      type: 'string',
      description: 'Filter by org type',
    },
    worked: {
      type: 'boolean',
      description: 'Filter to orgs where you worked',
    },
  },
  async run({ args }) {
    const filter: Record<string, string | number> = {}
    if (args.type) filter.org_type = args.type
    if (args.worked !== undefined) filter.worked = args.worked ? '1' : '0'

    const result = await forge.organizations.list(
      Object.keys(filter).length > 0 ? (filter as any) : undefined,
    )
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')

    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    if (result.data.length === 0) {
      console.log('No organizations found.')
      return
    }

    const idW = 10
    const nameW = 24
    const typeW = 12
    const industryW = 16
    const workedW = 8

    const header = [
      'ID'.padEnd(idW),
      'Name'.padEnd(nameW),
      'Type'.padEnd(typeW),
      'Industry'.padEnd(industryW),
      'Worked'.padEnd(workedW),
    ].join('  ')

    const divider = [
      '\u2500'.repeat(idW),
      '\u2500'.repeat(nameW),
      '\u2500'.repeat(typeW),
      '\u2500'.repeat(industryW),
      '\u2500'.repeat(workedW),
    ].join('  ')

    console.log(header)
    console.log(divider)

    for (const o of result.data) {
      const id = o.id.length > 8 ? o.id.slice(0, 6) + '..' : o.id
      const name = o.name.length > nameW ? o.name.slice(0, nameW - 1) + '\u2026' : o.name

      console.log(
        [
          id.padEnd(idW),
          name.padEnd(nameW),
          (o.org_type ?? '').padEnd(typeW),
          (o.industry ?? '').padEnd(industryW),
          (o.worked ? 'Yes' : 'No').padEnd(workedW),
        ].join('  '),
      )
    }

    console.log(`\n${result.data.length} of ${result.pagination.total} organizations`)
  },
})

const show = defineCommand({
  meta: { name: 'show', description: 'Show organization details' },
  args: {
    id: { type: 'positional', description: 'Organization ID', required: true },
  },
  async run({ args }) {
    const result = await forge.organizations.get(args.id)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')
    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    const o = result.data
    console.log(`Organization: ${o.name}`)
    console.log(`  ID:              ${o.id}`)
    console.log(`  Type:            ${o.org_type}`)
    console.log(`  Industry:        ${o.industry ?? '(none)'}`)
    console.log(`  Size:            ${o.size ?? '(none)'}`)
    console.log(`  Worked:          ${o.worked ? 'Yes' : 'No'}`)
    console.log(`  Employment Type: ${o.employment_type ?? '(none)'}`)
    console.log(`  Location:        ${o.location ?? '(none)'}`)
    console.log(`  Website:         ${o.website ?? '(none)'}`)
    if (o.notes) console.log(`  Notes:           ${o.notes}`)
  },
})

const edit = defineCommand({
  meta: { name: 'edit', description: 'Edit an organization' },
  args: {
    id: { type: 'positional', description: 'Organization ID', required: true },
    name: { type: 'string', description: 'New name' },
    type: { type: 'string', description: 'New org type' },
    industry: { type: 'string', description: 'New industry' },
    size: { type: 'string', description: 'New size' },
    worked: { type: 'boolean', description: 'Worked here?' },
    location: { type: 'string', description: 'New location' },
    website: { type: 'string', description: 'New website' },
  },
  async run({ args }) {
    const input: Record<string, unknown> = {}
    if (args.name) input.name = args.name
    if (args.type) input.org_type = args.type
    if (args.industry) input.industry = args.industry
    if (args.size) input.size = args.size
    if (args.worked !== undefined) input.worked = args.worked
    if (args.location) input.location = args.location
    if (args.website) input.website = args.website

    if (Object.keys(input).length === 0) {
      console.error('Error: No fields to update. Provide at least one flag.')
      process.exit(1)
    }

    const result = await forge.organizations.update(args.id, input as any)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    console.log(`Updated organization: ${result.data.name}`)
  },
})

const del = defineCommand({
  meta: { name: 'delete', description: 'Delete an organization' },
  args: {
    id: { type: 'positional', description: 'Organization ID', required: true },
  },
  async run({ args }) {
    const result = await forge.organizations.delete(args.id)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    console.log(`Deleted organization ${args.id}`)
  },
})

export const orgCommand = defineCommand({
  meta: { name: 'org', description: 'Manage organizations' },
  subCommands: {
    add,
    list,
    show,
    edit,
    delete: del,
  },
})
```

**Registration in `packages/cli/src/index.ts`:**
```typescript
import { orgCommand } from './commands/organization'
// ...
subCommands: {
  // ... existing ...
  org: orgCommand,
}
```

**Acceptance Criteria:**
- [ ] `forge org add --name "Anthropic" --type company --industry AI --worked` creates org
- [ ] `forge org list` shows table with ID, Name, Type, Industry, Worked columns
- [ ] `forge org list --type military` filters
- [ ] `forge org list --worked` filters to worked orgs
- [ ] `forge org show <id>` shows full org details
- [ ] `forge org edit <id> --industry "AI Safety"` updates
- [ ] `forge org delete <id>` deletes
- [ ] All commands support `--json` for machine-readable output

---

## Task 13.3: CLI Note Commands

**Goal:** CRUD for user notes with entity linking from the CLI.

**File:** `packages/cli/src/commands/note.ts` (NEW)

**Implementation:**

```typescript
import { defineCommand } from 'citty'
import { forge, exitOnNetworkError } from '../client'

const add = defineCommand({
  meta: { name: 'add', description: 'Add a new note' },
  args: {
    content: {
      type: 'string',
      description: 'Note content',
      required: true,
    },
    title: {
      type: 'string',
      description: 'Note title (optional)',
    },
  },
  async run({ args }) {
    const input: Record<string, string> = { content: args.content }
    if (args.title) input.title = args.title

    const result = await forge.notes.create(input as any)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const n = result.data
    console.log(`Created note: ${n.title ?? '(untitled)'}`)
    console.log(`  ID: ${n.id}`)
  },
})

const list = defineCommand({
  meta: { name: 'list', description: 'List notes' },
  args: {
    search: {
      type: 'string',
      description: 'Search notes by content',
    },
  },
  async run({ args }) {
    const filter: Record<string, string> = {}
    if (args.search) filter.search = args.search

    const result = await forge.notes.list(
      Object.keys(filter).length > 0 ? (filter as any) : undefined,
    )
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')
    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    if (result.data.length === 0) {
      console.log('No notes found.')
      return
    }

    for (const n of result.data) {
      const title = n.title ?? '(untitled)'
      const id = n.id.length > 8 ? n.id.slice(0, 6) + '..' : n.id
      const preview = n.content.length > 60 ? n.content.slice(0, 57) + '...' : n.content
      const refs = n.references.length > 0
        ? ` [${n.references.length} linked]`
        : ''

      console.log(`  [${id}] ${title}${refs}`)
      console.log(`          ${preview}`)
    }

    console.log(`\n${result.data.length} note(s)`)
  },
})

const show = defineCommand({
  meta: { name: 'show', description: 'Show note details' },
  args: {
    id: { type: 'positional', description: 'Note ID', required: true },
  },
  async run({ args }) {
    const result = await forge.notes.get(args.id)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')
    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    const n = result.data
    console.log(`Note: ${n.title ?? '(untitled)'}`)
    console.log(`  ID:      ${n.id}`)
    console.log(`  Created: ${n.created_at}`)
    console.log(`  Updated: ${n.updated_at}`)
    console.log(``)
    console.log(`  ${n.content}`)

    if (n.references.length > 0) {
      console.log(``)
      console.log(`  Linked entities:`)
      for (const ref of n.references) {
        console.log(`    - ${ref.entity_type}: ${ref.entity_id}`)
      }
    }
  },
})

const edit = defineCommand({
  meta: { name: 'edit', description: 'Edit a note' },
  args: {
    id: { type: 'positional', description: 'Note ID', required: true },
    title: { type: 'string', description: 'New title' },
    content: { type: 'string', description: 'New content' },
  },
  async run({ args }) {
    const input: Record<string, string> = {}
    if (args.title) input.title = args.title
    if (args.content) input.content = args.content

    if (Object.keys(input).length === 0) {
      console.error('Error: No fields to update. Provide --title or --content.')
      process.exit(1)
    }

    const result = await forge.notes.update(args.id, input as any)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    console.log(`Updated note: ${result.data.title ?? '(untitled)'}`)
  },
})

const del = defineCommand({
  meta: { name: 'delete', description: 'Delete a note' },
  args: {
    id: { type: 'positional', description: 'Note ID', required: true },
  },
  async run({ args }) {
    const result = await forge.notes.delete(args.id)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    console.log(`Deleted note ${args.id}`)
  },
})

const link = defineCommand({
  meta: { name: 'link', description: 'Link a note to an entity' },
  args: {
    id: { type: 'positional', description: 'Note ID', required: true },
    'entity-type': {
      type: 'string',
      description: 'Entity type (source, bullet, perspective, resume_entry, resume, skill, organization)',
      required: true,
    },
    'entity-id': {
      type: 'string',
      description: 'Entity ID',
      required: true,
    },
  },
  async run({ args }) {
    const result = await forge.notes.addReference(args.id, {
      entity_type: args['entity-type'],
      entity_id: args['entity-id'],
    })
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    console.log(`Linked note ${args.id} to ${args['entity-type']} ${args['entity-id']}`)
  },
})

export const noteCommand = defineCommand({
  meta: { name: 'note', description: 'Manage user notes' },
  subCommands: {
    add,
    list,
    show,
    edit,
    delete: del,
    link,
  },
})
```

**Registration in `packages/cli/src/index.ts`:**
```typescript
import { noteCommand } from './commands/note'
// ...
subCommands: {
  // ... existing ...
  note: noteCommand,
}
```

**Acceptance Criteria:**
- [ ] `forge note add --content "Remember to ..."` creates note
- [ ] `forge note add --title "Interview Prep" --content "..."` creates titled note
- [ ] `forge note list` shows all notes with previews
- [ ] `forge note list --search kubernetes` searches by content
- [ ] `forge note show <id>` shows full content + linked entities
- [ ] `forge note edit <id> --content "Updated"` updates content
- [ ] `forge note delete <id>` deletes note
- [ ] `forge note link <note-id> --entity-type source --entity-id <uuid>` links entity

---

## Task 13.4: CLI Skills Commands

**Goal:** CRUD for skills from the CLI.

**File:** `packages/cli/src/commands/skill.ts` (NEW)

**Implementation:**

```typescript
import { defineCommand } from 'citty'
import { forge, exitOnNetworkError } from '../client'

const add = defineCommand({
  meta: { name: 'add', description: 'Add a new skill' },
  args: {
    name: {
      type: 'string',
      description: 'Skill name',
      required: true,
    },
    category: {
      type: 'string',
      description: 'Category (ai_ml, cloud, database, devops, frameworks, general, language, languages, os, security, tools)',
    },
  },
  async run({ args }) {
    const input: Record<string, string> = { name: args.name }
    if (args.category) input.category = args.category

    // Uses the supporting routes endpoint
    const result = await forge.request<any>('POST', '/api/skills', input)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const s = result.data
    console.log(`Created skill: ${s.name}`)
    console.log(`  ID:       ${s.id}`)
    console.log(`  Category: ${s.category ?? '(none)'}`)
  },
})

const list = defineCommand({
  meta: { name: 'list', description: 'List skills' },
  args: {
    category: {
      type: 'string',
      description: 'Filter by category',
    },
  },
  async run({ args }) {
    const params: Record<string, string> = {}
    if (args.category) params.category = args.category

    const qs = Object.keys(params).length > 0
      ? '?' + new URLSearchParams(params).toString()
      : ''

    // Uses the supporting routes endpoint
    const result = await forge.request<any[]>('GET', `/api/skills${qs}`)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')
    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    if (result.data.length === 0) {
      console.log('No skills found.')
      return
    }

    const idW = 10
    const nameW = 28
    const catW = 16

    const header = [
      'ID'.padEnd(idW),
      'Name'.padEnd(nameW),
      'Category'.padEnd(catW),
    ].join('  ')

    const divider = [
      '\u2500'.repeat(idW),
      '\u2500'.repeat(nameW),
      '\u2500'.repeat(catW),
    ].join('  ')

    console.log(header)
    console.log(divider)

    for (const s of result.data) {
      const id = s.id.length > 8 ? s.id.slice(0, 6) + '..' : s.id
      const name = s.name.length > nameW ? s.name.slice(0, nameW - 1) + '\u2026' : s.name

      console.log(
        [
          id.padEnd(idW),
          name.padEnd(nameW),
          (s.category ?? '').padEnd(catW),
        ].join('  '),
      )
    }

    console.log(`\n${result.data.length} skill(s)`)
  },
})

const show = defineCommand({
  meta: { name: 'show', description: 'Show skill details' },
  args: {
    id: { type: 'positional', description: 'Skill ID', required: true },
  },
  async run({ args }) {
    const result = await forge.request<any>('GET', `/api/skills/${args.id}`)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    const useJson = process.argv.includes('--json')
    if (useJson) {
      console.log(JSON.stringify(result.data, null, 2))
      return
    }

    const s = result.data
    console.log(`Skill: ${s.name}`)
    console.log(`  ID:       ${s.id}`)
    console.log(`  Category: ${s.category ?? '(none)'}`)
    if (s.notes) console.log(`  Notes:    ${s.notes}`)
    console.log(`  Created:  ${s.created_at}`)
  },
})

const del = defineCommand({
  meta: { name: 'delete', description: 'Delete a skill' },
  args: {
    id: { type: 'positional', description: 'Skill ID', required: true },
  },
  async run({ args }) {
    const result = await forge.request<void>('DELETE', `/api/skills/${args.id}`)
    if (!result.ok) {
      exitOnNetworkError(result.error)
      console.error(`Error: ${result.error.message}`)
      process.exit(1)
    }

    console.log(`Deleted skill ${args.id}`)
  },
})

export const skillCommand = defineCommand({
  meta: { name: 'skill', description: 'Manage skills' },
  subCommands: {
    add,
    list,
    show,
    delete: del,
  },
})
```

**Note:** The skill commands call the existing supporting routes (`/api/skills`). If the SDK gains a dedicated `SkillsResource`, these can be refactored to use `forge.skills.*` instead of `forge.request()` directly. For now, the direct `request()` approach works.

**Registration in `packages/cli/src/index.ts`:**
```typescript
import { skillCommand } from './commands/skill'
// ...
subCommands: {
  // ... existing ...
  skill: skillCommand,
}
```

**Acceptance Criteria:**
- [ ] `forge skill add --name Kubernetes --category devops` creates skill
- [ ] `forge skill list` shows table
- [ ] `forge skill list --category security` filters
- [ ] `forge skill show <id>` shows details
- [ ] `forge skill delete <id>` deletes
- [ ] `--json` works on all commands

---

## Task 13.5: v1 Import Command

**Goal:** Implement `forge import v1 <path-to-v1-db>` to non-destructively migrate all v1 data into Forge. This is the most complex task in Phase 13.

**File:** `packages/cli/src/commands/import.ts` (NEW)

**Design decisions:**
- Opens v1 DB read-only via `new Database(path, { readonly: true })`
- Uses **direct DB access** for the Forge database (not SDK/API) for performance — bulk inserts with transactions are much faster than individual HTTP calls for 200+ entities
- Idempotent via `v1_import_map` — checks before each insert, skips if already imported
- Prints running progress and summary at end
- Handles errors gracefully — logs failures per entity but continues

**Implementation:**

```typescript
import { defineCommand } from 'citty'
import { Database } from 'bun:sqlite'

// ── Types ───────────────────────────────────────────────────────────

interface ImportSummary {
  organizations: { imported: number; skipped: number; failed: number }
  skills: { imported: number; skipped: number; failed: number }
  languages: { imported: number; skipped: number; failed: number }
  roles: { imported: number; skipped: number; failed: number }
  education: { imported: number; skipped: number; failed: number }
  clearances: { imported: number; skipped: number; failed: number }
  bullets: { imported: number; skipped: number; failed: number }
  bullet_sources: { imported: number; skipped: number; failed: number }
  bullet_skills: { imported: number; skipped: number; failed: number }
  resumes: { imported: number; skipped: number; failed: number }
  perspectives: { imported: number; skipped: number; failed: number }
  resume_entries: { imported: number; skipped: number; failed: number }
}

function newCounter() {
  return { imported: 0, skipped: 0, failed: 0 }
}

// ── UUID helper ─────────────────────────────────────────────────────

function uuid(): string {
  return crypto.randomUUID()
}

function now(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')
}

// ── Import map helpers ──────────────────────────────────────────────

function getForgeId(forgeDb: Database, entityType: string, v1Id: number): string | null {
  const row = forgeDb.query(
    'SELECT forge_id FROM v1_import_map WHERE v1_entity_type = ? AND v1_id = ?'
  ).get(entityType, v1Id) as { forge_id: string } | null
  return row?.forge_id ?? null
}

function recordMapping(forgeDb: Database, entityType: string, v1Id: number, forgeId: string) {
  forgeDb.query(
    'INSERT INTO v1_import_map (v1_entity_type, v1_id, forge_id) VALUES (?, ?, ?)'
  ).run(entityType, v1Id, forgeId)
}

// ── Import functions ────────────────────────────────────────────────

function importOrganizations(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing organizations...')
  const rows = v1.query('SELECT * FROM organizations').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'organization', row.id)
      if (existing) {
        summary.organizations.skipped++
        continue
      }

      const id = uuid()
      forge.query(`
        INSERT INTO organizations (
          id, name, org_type, industry, size, worked,
          employment_type, location, headquarters, website,
          linkedin_url, glassdoor_url, glassdoor_rating,
          reputation_notes, notes, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).run(
        id,
        row.name,
        row.org_type ?? 'company',
        row.industry ?? null,
        row.size ?? null,
        row.worked ? 1 : 0,
        row.employment_type ?? null,
        row.location ?? null,
        row.headquarters ?? null,
        row.website ?? null,
        row.linkedin_url ?? null,
        row.glassdoor_url ?? null,
        row.glassdoor_rating ?? null,
        row.reputation_notes ?? null,
        row.notes ?? null,
        now(),
        now(),
      )
      recordMapping(forge, 'organization', row.id, id)
      summary.organizations.imported++
    } catch (err) {
      console.error(`    Failed to import organization ${row.id} (${row.name}): ${err}`)
      summary.organizations.failed++
    }
  }
  console.log(`    ${summary.organizations.imported} imported, ${summary.organizations.skipped} skipped, ${summary.organizations.failed} failed`)
}

function importSkills(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing skills...')
  const rows = v1.query('SELECT * FROM skills').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'skill', row.id)
      if (existing) {
        summary.skills.skipped++
        continue
      }

      const id = uuid()
      forge.query(`
        INSERT INTO skills (id, name, category, notes, created_at)
        VALUES (?, ?, ?, ?, ?)
      `).run(id, row.name, row.category ?? null, row.notes ?? null, now())
      recordMapping(forge, 'skill', row.id, id)
      summary.skills.imported++
    } catch (err) {
      console.error(`    Failed to import skill ${row.id} (${row.name}): ${err}`)
      summary.skills.failed++
    }
  }
  console.log(`    ${summary.skills.imported} imported, ${summary.skills.skipped} skipped, ${summary.skills.failed} failed`)
}

function importLanguages(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing languages as skills...')
  const rows = v1.query('SELECT * FROM languages').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'language', row.id)
      if (existing) {
        summary.languages.skipped++
        continue
      }

      const id = uuid()
      // Proficiency info stored in notes
      const notes = [
        row.proficiency ? `Proficiency: ${row.proficiency}` : null,
        row.reading_proficiency ? `Reading: ${row.reading_proficiency}` : null,
        row.writing_proficiency ? `Writing: ${row.writing_proficiency}` : null,
        row.certification ? `Certification: ${row.certification}` : null,
        row.notes,
      ].filter(Boolean).join('. ')

      forge.query(`
        INSERT INTO skills (id, name, category, notes, created_at)
        VALUES (?, ?, 'language', ?, ?)
      `).run(id, row.name, notes || null, now())
      recordMapping(forge, 'language', row.id, id)
      summary.languages.imported++
    } catch (err) {
      console.error(`    Failed to import language ${row.id} (${row.name}): ${err}`)
      summary.languages.failed++
    }
  }
  console.log(`    ${summary.languages.imported} imported, ${summary.languages.skipped} skipped, ${summary.languages.failed} failed`)
}

function importRoles(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing roles as sources + source_roles...')
  const rows = v1.query('SELECT * FROM roles').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'role', row.id)
      if (existing) {
        summary.roles.skipped++
        continue
      }

      const id = uuid()

      // Generate source description by concatenating linked bullet contents
      const bulletContents = v1.query(`
        SELECT b.content FROM bullets b
        JOIN bullet_roles br ON b.id = br.bullet_id
        WHERE br.role_id = ?
        ORDER BY b.id
      `).all(row.id) as any[]

      const description = bulletContents.length > 0
        ? bulletContents.map((b: any) => b.content).join('\n\n')
        : row.notes ?? `Role: ${row.title}`

      // Map employer_id to organization_id via import map
      let orgId: string | null = null
      if (row.employer_id) {
        orgId = getForgeId(forge, 'organization', row.employer_id)
      }

      // Create source (base row)
      forge.query(`
        INSERT INTO sources (
          id, title, description, source_type, notes,
          status, updated_by, created_at, updated_at
        ) VALUES (?, ?, ?, 'role', ?, 'approved', 'human', ?, ?)
      `).run(id, row.title, description, row.notes ?? null, now(), now())

      // Create source_roles extension
      forge.query(`
        INSERT INTO source_roles (
          source_id, organization_id, start_date, end_date,
          is_current, work_arrangement, base_salary, total_comp_notes
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(
        id,
        orgId,
        row.start_date ?? null,
        row.end_date ?? null,
        row.is_current ? 1 : 0,
        row.work_arrangement ?? null,
        row.base_salary ?? null,
        row.total_comp_notes ?? null,
      )

      recordMapping(forge, 'role', row.id, id)
      summary.roles.imported++
    } catch (err) {
      console.error(`    Failed to import role ${row.id} (${row.title}): ${err}`)
      summary.roles.failed++
    }
  }
  console.log(`    ${summary.roles.imported} imported, ${summary.roles.skipped} skipped, ${summary.roles.failed} failed`)
}

function importEducation(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing education as sources + source_education...')
  const rows = v1.query('SELECT * FROM education').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'education', row.id)
      if (existing) {
        summary.education.skipped++
        continue
      }

      const id = uuid()

      // Description from notes or generated
      const description = row.notes ?? `${row.type}: ${row.name}${row.institution ? ' at ' + row.institution : ''}`

      // education_type mapping: v1 'certificate' -> 'certificate', 'degree' -> 'degree'
      const educationType = row.type === 'certificate' ? 'certificate'
        : row.type === 'degree' ? 'degree'
        : row.type === 'course' ? 'course'
        : row.type === 'self_taught' ? 'self_taught'
        : 'certificate'  // fallback

      // issuing_body derived from institution for certs
      const issuingBody = row.type === 'certificate' ? row.institution : null

      forge.query(`
        INSERT INTO sources (
          id, title, description, source_type, notes,
          status, updated_by, created_at, updated_at
        ) VALUES (?, ?, ?, 'education', ?, 'approved', 'human', ?, ?)
      `).run(id, row.name, description, row.notes ?? null, now(), now())

      forge.query(`
        INSERT INTO source_education (
          source_id, education_type, institution, field,
          start_date, end_date, is_in_progress,
          credential_id, expiration_date, issuing_body, url
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).run(
        id,
        educationType,
        row.institution ?? null,
        row.field ?? null,
        row.start_date ?? null,
        row.end_date ?? null,
        row.is_in_progress ? 1 : 0,
        row.credential_id ?? null,
        row.expiration_date ?? null,
        issuingBody,
        row.url ?? null,
      )

      recordMapping(forge, 'education', row.id, id)
      summary.education.imported++
    } catch (err) {
      console.error(`    Failed to import education ${row.id} (${row.name}): ${err}`)
      summary.education.failed++
    }
  }
  console.log(`    ${summary.education.imported} imported, ${summary.education.skipped} skipped, ${summary.education.failed} failed`)
}

function importClearances(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing clearances as sources + source_clearances...')
  const rows = v1.query('SELECT * FROM clearances').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'clearance', row.id)
      if (existing) {
        summary.clearances.skipped++
        continue
      }

      const id = uuid()

      const title = `${row.level} Security Clearance`
      const description = row.notes ?? `${row.level} clearance${row.polygraph ? ' with ' + row.polygraph + ' polygraph' : ''}`

      forge.query(`
        INSERT INTO sources (
          id, title, description, source_type, notes,
          status, updated_by, created_at, updated_at
        ) VALUES (?, ?, ?, 'clearance', ?, 'approved', 'human', ?, ?)
      `).run(id, title, description, row.notes ?? null, now(), now())

      forge.query(`
        INSERT INTO source_clearances (
          source_id, level, polygraph, status,
          sponsoring_agency, investigation_date,
          adjudication_date, reinvestigation_date, read_on
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).run(
        id,
        row.level,
        row.polygraph ?? null,
        row.status ?? null,
        row.sponsoring_agency ?? null,
        row.investigation_date ?? null,
        row.adjudication_date ?? null,
        row.reinvestigation_date ?? null,
        row.read_on ?? null,
      )

      recordMapping(forge, 'clearance', row.id, id)
      summary.clearances.imported++
    } catch (err) {
      console.error(`    Failed to import clearance ${row.id}: ${err}`)
      summary.clearances.failed++
    }
  }
  console.log(`    ${summary.clearances.imported} imported, ${summary.clearances.skipped} skipped, ${summary.clearances.failed} failed`)
}

function importBullets(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing bullets...')
  const rows = v1.query('SELECT * FROM bullets').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'bullet', row.id)
      if (existing) {
        summary.bullets.skipped++
        continue
      }

      const id = uuid()

      // Get primary source description for source_content_snapshot
      // Find the primary role for this bullet
      const primaryRole = v1.query(`
        SELECT r.title, r.notes FROM bullet_roles br
        JOIN roles r ON br.role_id = r.id
        WHERE br.bullet_id = ? AND br.is_primary = 1
        LIMIT 1
      `).get(row.id) as any | null

      // If no primary role, try any role
      const anyRole = primaryRole ?? (v1.query(`
        SELECT r.title, r.notes FROM bullet_roles br
        JOIN roles r ON br.role_id = r.id
        WHERE br.bullet_id = ?
        LIMIT 1
      `).get(row.id) as any | null)

      // Snapshot is primary source's description at import time
      // Since we generate descriptions from bullet contents, use the role title + notes
      const snapshot = anyRole?.notes ?? anyRole?.title ?? ''

      // v1 framing values become domain on bullets
      // Values: ai_ml, devops, leadership, security, software_engineering, systems_engineering
      const domain = row.framing ?? null

      forge.query(`
        INSERT INTO bullets (
          id, content, source_content_snapshot,
          domain, notes, status, updated_by,
          created_at
        ) VALUES (?, ?, ?, ?, ?, 'approved', 'human', ?)
      `).run(
        id,
        row.content,
        snapshot,
        domain,
        row.notes ?? null,
        now(),
      )

      recordMapping(forge, 'bullet', row.id, id)
      summary.bullets.imported++
    } catch (err) {
      console.error(`    Failed to import bullet ${row.id}: ${err}`)
      summary.bullets.failed++
    }
  }
  console.log(`    ${summary.bullets.imported} imported, ${summary.bullets.skipped} skipped, ${summary.bullets.failed} failed`)
}

function importBulletSources(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing bullet_sources from bullet_roles...')

  // bullet_roles -> bullet_sources
  const roleRows = v1.query('SELECT * FROM bullet_roles').all() as any[]
  for (const row of roleRows) {
    try {
      const bulletId = getForgeId(forge, 'bullet', row.bullet_id)
      const sourceId = getForgeId(forge, 'role', row.role_id)

      if (!bulletId || !sourceId) {
        console.error(`    Skipping bullet_role (${row.bullet_id}, ${row.role_id}): missing mapping`)
        summary.bullet_sources.failed++
        continue
      }

      // Check if already exists (idempotency)
      const exists = forge.query(
        'SELECT 1 FROM bullet_sources WHERE bullet_id = ? AND source_id = ?'
      ).get(bulletId, sourceId)
      if (exists) {
        summary.bullet_sources.skipped++
        continue
      }

      forge.query(`
        INSERT INTO bullet_sources (bullet_id, source_id, is_primary)
        VALUES (?, ?, ?)
      `).run(bulletId, sourceId, row.is_primary ? 1 : 0)
      summary.bullet_sources.imported++
    } catch (err) {
      console.error(`    Failed to import bullet_role (${row.bullet_id}, ${row.role_id}): ${err}`)
      summary.bullet_sources.failed++
    }
  }

  // bullet_education -> bullet_sources (is_primary = 0)
  const eduRows = v1.query(`
    SELECT * FROM bullet_education
  `).all() as any[]
  for (const row of eduRows) {
    try {
      const bulletId = getForgeId(forge, 'bullet', row.bullet_id)
      const sourceId = getForgeId(forge, 'education', row.education_id)

      if (!bulletId || !sourceId) {
        summary.bullet_sources.failed++
        continue
      }

      const exists = forge.query(
        'SELECT 1 FROM bullet_sources WHERE bullet_id = ? AND source_id = ?'
      ).get(bulletId, sourceId)
      if (exists) {
        summary.bullet_sources.skipped++
        continue
      }

      forge.query(`
        INSERT INTO bullet_sources (bullet_id, source_id, is_primary)
        VALUES (?, ?, 0)
      `).run(bulletId, sourceId)
      summary.bullet_sources.imported++
    } catch (err) {
      summary.bullet_sources.failed++
    }
  }

  console.log(`    ${summary.bullet_sources.imported} imported, ${summary.bullet_sources.skipped} skipped, ${summary.bullet_sources.failed} failed`)
}

function importBulletSkills(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing bullet_skills...')
  const rows = v1.query('SELECT * FROM bullet_skills').all() as any[]

  for (const row of rows) {
    try {
      const bulletId = getForgeId(forge, 'bullet', row.bullet_id)
      const skillId = getForgeId(forge, 'skill', row.skill_id)

      if (!bulletId || !skillId) {
        summary.bullet_skills.failed++
        continue
      }

      const exists = forge.query(
        'SELECT 1 FROM bullet_skills WHERE bullet_id = ? AND skill_id = ?'
      ).get(bulletId, skillId)
      if (exists) {
        summary.bullet_skills.skipped++
        continue
      }

      forge.query(`
        INSERT INTO bullet_skills (bullet_id, skill_id)
        VALUES (?, ?)
      `).run(bulletId, skillId)
      summary.bullet_skills.imported++
    } catch (err) {
      summary.bullet_skills.failed++
    }
  }
  console.log(`    ${summary.bullet_skills.imported} imported, ${summary.bullet_skills.skipped} skipped, ${summary.bullet_skills.failed} failed`)
}

function importResumes(v1: Database, forge: Database, summary: ImportSummary) {
  console.log('\n  Importing resumes...')
  const rows = v1.query('SELECT * FROM resumes').all() as any[]

  for (const row of rows) {
    try {
      const existing = getForgeId(forge, 'resume', row.id)
      if (existing) {
        summary.resumes.skipped++
        continue
      }

      const id = uuid()

      // v1 target_company -> target_employer
      // archetype inferred from name or set to null
      const archetype = inferArchetypeFromName(row.name)

      forge.query(`
        INSERT INTO resumes (
          id, name, target_role, target_employer,
          archetype, notes, status, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, 'draft', ?, ?)
      `).run(
        id,
        row.name,
        row.target_role ?? '',
        row.target_company ?? '',
        archetype,
        row.notes ?? null,
        now(),
        now(),
      )

      recordMapping(forge, 'resume', row.id, id)
      summary.resumes.imported++
    } catch (err) {
      console.error(`    Failed to import resume ${row.id} (${row.name}): ${err}`)
      summary.resumes.failed++
    }
  }
  console.log(`    ${summary.resumes.imported} imported, ${summary.resumes.skipped} skipped, ${summary.resumes.failed} failed`)
}

/**
 * Create synthetic identity perspectives for resume_bullets.
 *
 * For each v1 resume_bullet, we need a perspective in between (Forge requires
 * the derivation chain: source -> bullet -> perspective -> resume_entry).
 *
 * Synthetic perspective:
 * - content = bullet content (identity transformation)
 * - bullet_content_snapshot = bullet content
 * - framing = 'accomplishment' (default)
 * - target_archetype = null (not targeted)
 * - domain = bullet's domain/framing value
 * - status = 'approved'
 */
function importResumeBulletsAsSyntheticEntries(
  v1: Database, forge: Database, summary: ImportSummary,
) {
  console.log('\n  Creating synthetic perspectives for resume_bullets...')
  const rows = v1.query('SELECT * FROM resume_bullets ORDER BY resume_id, position').all() as any[]

  for (const row of rows) {
    try {
      const resumeId = getForgeId(forge, 'resume', row.resume_id)
      const bulletId = getForgeId(forge, 'bullet', row.bullet_id)

      if (!resumeId || !bulletId) {
        console.error(`    Skipping resume_bullet (${row.resume_id}, ${row.bullet_id}): missing mapping`)
        summary.perspectives.failed++
        summary.resume_entries.failed++
        continue
      }

      // Check if we already created a synthetic perspective for this combination
      // Use a compound key: resume_bullet:{resume_id}:{bullet_id}
      const mapKey = row.resume_id * 10000 + row.bullet_id  // compound integer key
      const existingPerspId = getForgeId(forge, `resume_bullet_perspective`, mapKey)
      const existingEntryId = getForgeId(forge, `resume_bullet_entry`, mapKey)

      if (existingPerspId && existingEntryId) {
        summary.perspectives.skipped++
        summary.resume_entries.skipped++
        continue
      }

      // Get bullet content for the synthetic perspective
      const bullet = forge.query('SELECT content, domain FROM bullets WHERE id = ?').get(bulletId) as any
      if (!bullet) {
        summary.perspectives.failed++
        summary.resume_entries.failed++
        continue
      }

      // Step 1: Create synthetic identity perspective
      const perspId = uuid()

      if (!existingPerspId) {
        forge.query(`
          INSERT INTO perspectives (
            id, bullet_id, content, bullet_content_snapshot,
            target_archetype, domain, framing,
            notes, status, created_at
          ) VALUES (?, ?, ?, ?, NULL, ?, 'accomplishment', NULL, 'approved', ?)
        `).run(
          perspId,
          bulletId,
          bullet.content,       // identity: same as bullet
          bullet.content,       // snapshot matches
          bullet.domain ?? null,
          now(),
        )
        recordMapping(forge, `resume_bullet_perspective`, mapKey, perspId)
        summary.perspectives.imported++
      }

      const finalPerspId = existingPerspId ?? perspId

      // Step 2: Create resume_entry referencing this perspective
      if (!existingEntryId) {
        const entryId = uuid()

        // v1 sections (ai_engineering, devops, government, etc.) all map to work_history
        // Original section value preserved in notes
        const v1Section = row.section ?? 'work_history'
        const section = 'work_history'
        const entryNotes = v1Section !== 'work_history'
          ? `v1_section: ${v1Section}`
          : null

        forge.query(`
          INSERT INTO resume_entries (
            id, resume_id, perspective_id, content,
            perspective_content_snapshot, section, position,
            notes, created_at, updated_at
          ) VALUES (?, ?, ?, NULL, NULL, ?, ?, ?, ?, ?)
        `).run(
          entryId,
          resumeId,
          finalPerspId,
          section,
          row.position ?? 0,
          entryNotes,
          now(),
          now(),
        )
        recordMapping(forge, `resume_bullet_entry`, mapKey, entryId)
        summary.resume_entries.imported++
      }
    } catch (err) {
      console.error(`    Failed to import resume_bullet (${row.resume_id}, ${row.bullet_id}): ${err}`)
      summary.perspectives.failed++
      summary.resume_entries.failed++
    }
  }
  console.log(`    Perspectives: ${summary.perspectives.imported} imported, ${summary.perspectives.skipped} skipped, ${summary.perspectives.failed} failed`)
  console.log(`    Entries: ${summary.resume_entries.imported} imported, ${summary.resume_entries.skipped} skipped, ${summary.resume_entries.failed} failed`)
}

// ── Archetype inference ─────────────────────────────────────────────

function inferArchetypeFromName(name: string): string | null {
  const lower = name.toLowerCase()
  if (lower.includes('ai') || lower.includes('ml') || lower.includes('machine learning')) {
    return 'agentic-ai'
  }
  if (lower.includes('security') || lower.includes('cyber')) {
    return 'security-engineer'
  }
  if (lower.includes('devops') || lower.includes('platform') || lower.includes('sre')) {
    return 'platform-engineer'
  }
  if (lower.includes('cloud') || lower.includes('infrastructure')) {
    return 'cloud-engineer'
  }
  return null
}

// ── Main import command ─────────────────────────────────────────────

const v1Import = defineCommand({
  meta: { name: 'v1', description: 'Import data from v1 SQLite database' },
  args: {
    path: {
      type: 'positional',
      description: 'Path to v1 SQLite database file',
      required: true,
    },
    'forge-db': {
      type: 'string',
      description: 'Path to Forge database (default: from FORGE_DB_PATH env)',
    },
  },
  async run({ args }) {
    const v1Path = args.path
    const forgeDbPath = args['forge-db'] ?? process.env.FORGE_DB_PATH

    if (!forgeDbPath) {
      console.error('Error: Forge database path required. Set FORGE_DB_PATH or use --forge-db.')
      process.exit(1)
    }

    console.log(`\nForge v1 Import`)
    console.log(`  v1 database:    ${v1Path}`)
    console.log(`  Forge database: ${forgeDbPath}`)

    // Open databases
    let v1: Database
    try {
      v1 = new Database(v1Path, { readonly: true })
    } catch (err) {
      console.error(`Error: Could not open v1 database at ${v1Path}: ${err}`)
      process.exit(1)
    }

    let forgeDb: Database
    try {
      forgeDb = new Database(forgeDbPath)
    } catch (err) {
      console.error(`Error: Could not open Forge database at ${forgeDbPath}: ${err}`)
      v1.close()
      process.exit(1)
    }

    // Enable WAL mode for better write performance
    forgeDb.exec('PRAGMA journal_mode = WAL')
    forgeDb.exec('PRAGMA foreign_keys = ON')

    const summary: ImportSummary = {
      organizations: newCounter(),
      skills: newCounter(),
      languages: newCounter(),
      roles: newCounter(),
      education: newCounter(),
      clearances: newCounter(),
      bullets: newCounter(),
      bullet_sources: newCounter(),
      bullet_skills: newCounter(),
      resumes: newCounter(),
      perspectives: newCounter(),
      resume_entries: newCounter(),
    }

    try {
      console.log('\nStarting import...')

      // Migration order from spec section 3:
      // 1. organizations (no dependencies)
      importOrganizations(v1, forgeDb, summary)

      // 2. skills + languages as skills (no dependencies)
      importSkills(v1, forgeDb, summary)
      importLanguages(v1, forgeDb, summary)

      // 3. sources + source_roles (depends on organizations)
      importRoles(v1, forgeDb, summary)

      // 4. sources + source_education
      importEducation(v1, forgeDb, summary)

      // 5. sources + source_clearances
      importClearances(v1, forgeDb, summary)

      // 6. bullets (no source_id, just the bullet itself)
      importBullets(v1, forgeDb, summary)

      // 7. bullet_sources junction (depends on bullets + sources)
      importBulletSources(v1, forgeDb, summary)

      // 8. bullet_skills (depends on bullets + skills)
      importBulletSkills(v1, forgeDb, summary)

      // 9. resumes (no data dependencies)
      importResumes(v1, forgeDb, summary)

      // 10-11. Synthetic perspectives + resume_entries (depends on resumes + bullets)
      importResumeBulletsAsSyntheticEntries(v1, forgeDb, summary)

      // Print summary
      console.log('\n' + '='.repeat(60))
      console.log('Import Summary')
      console.log('='.repeat(60))

      const entries = Object.entries(summary) as [string, { imported: number; skipped: number; failed: number }][]
      const maxLabel = Math.max(...entries.map(([k]) => k.length))

      for (const [entity, counts] of entries) {
        const total = counts.imported + counts.skipped + counts.failed
        if (total === 0) continue
        const label = entity.padEnd(maxLabel)
        console.log(
          `  ${label}  ${String(counts.imported).padStart(4)} imported  ${String(counts.skipped).padStart(4)} skipped  ${String(counts.failed).padStart(4)} failed`,
        )
      }

      const totalImported = entries.reduce((sum, [, c]) => sum + c.imported, 0)
      const totalFailed = entries.reduce((sum, [, c]) => sum + c.failed, 0)

      console.log('')
      if (totalFailed > 0) {
        console.log(`  Total: ${totalImported} imported, ${totalFailed} failed`)
        console.log('  Some entities failed to import. Check errors above.')
      } else {
        console.log(`  Total: ${totalImported} entities imported successfully.`)
      }
      console.log('')
    } finally {
      v1.close()
      forgeDb.close()
    }
  },
})

export const importCommand = defineCommand({
  meta: { name: 'import', description: 'Import data from external sources' },
  subCommands: {
    v1: v1Import,
  },
})
```

**Registration in `packages/cli/src/index.ts`:**
```typescript
import { importCommand } from './commands/import'
// ...
subCommands: {
  // ... existing ...
  import: importCommand,
}
```

**Key data transformation logic explained:**

1. **UUID mapping:** Each v1 integer ID is mapped to a new UUID via `v1_import_map(v1_entity_type, v1_id) -> forge_id`. The `getForgeId()` helper looks up the mapping. The `recordMapping()` helper stores new mappings after successful inserts.

2. **Source description generation for roles:** For each v1 role, the description is generated by concatenating the contents of all bullets linked to that role via `bullet_roles`. This provides a narrative summary of the role. If no bullets exist, falls back to `role.notes` or `"Role: {title}"`.

3. **Synthetic perspective creation:** Forge requires the full derivation chain (source -> bullet -> perspective -> resume_entry). v1 links bullets directly to resumes. For each `resume_bullet`, the import creates:
   - A **synthetic identity perspective**: content = bullet content (unchanged), `bullet_content_snapshot` = bullet content (matches), framing = `'accomplishment'` (default), status = `'approved'`
   - A **resume_entry** in reference mode (`content = NULL`) pointing to the synthetic perspective

4. **Resume section mapping:** v1 sections are domain-specific (`ai_engineering`, `devops`, `government`, etc.). These all map to `work_history` in Forge (they are all work experience bullets). The original v1 section value is preserved in the resume_entry's `notes` field as `"v1_section: {value}"` for reference.

**Acceptance Criteria:**
- [ ] `forge import v1 <path>` opens v1 DB read-only
- [ ] v1 DB is not modified (read-only open)
- [ ] All 18 organizations imported with correct field mapping
- [ ] All 60 skills imported with categories
- [ ] 2 languages imported as skills with `category = 'language'`, proficiency in notes
- [ ] All 13 roles imported as sources + source_roles with organization linkage
- [ ] Role source descriptions are concatenated bullet contents
- [ ] All 22 education entries imported as sources + source_education
- [ ] 1 clearance imported as source + source_clearance
- [ ] All 73 bullets imported with `status = 'approved'`, `updated_by = 'human'`
- [ ] `bullets.domain` populated from v1 `framing` field
- [ ] 3 hierarchical bullets (parent_id) imported as independent bullets (parent_id dropped)
- [ ] All bullet_roles mapped to bullet_sources with `is_primary` preserved
- [ ] All bullet_education mapped to bullet_sources with `is_primary = 0`
- [ ] All 97 bullet_skills associations preserved
- [ ] All 3 resumes imported with `target_company` -> `target_employer`
- [ ] All 55 resume_bullets imported via synthetic perspectives -> resume_entries
- [ ] Synthetic perspectives have content = bullet content, status = approved
- [ ] Resume entries have `section = 'work_history'`, original v1 section in notes
- [ ] Import is idempotent — running twice produces zero new imports on second run
- [ ] Summary printed at end with counts per entity type
- [ ] Individual failures are logged but do not abort the full import

---

## Task 13.6: Import Integration Test

**Goal:** End-to-end test proving the import is correct and idempotent.

**File:** `packages/cli/src/commands/__tests__/import.test.ts` (NEW)

**Test approach:** Create a test v1-like SQLite database with representative sample data, run the import logic against it, verify all entities transferred correctly, then run again and verify no duplicates.

**Implementation:**

```typescript
import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { Database } from 'bun:sqlite'

describe('v1 Import', () => {
  let v1: Database
  let forgeDb: Database

  beforeEach(() => {
    // Create v1 test database with sample data
    v1 = new Database(':memory:')
    v1.exec(`
      CREATE TABLE organizations (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        org_type TEXT,
        industry TEXT,
        size TEXT,
        worked INTEGER DEFAULT 0,
        employment_type TEXT,
        location TEXT,
        headquarters TEXT,
        website TEXT,
        linkedin_url TEXT,
        glassdoor_url TEXT,
        glassdoor_rating REAL,
        reputation_notes TEXT,
        notes TEXT
      );

      CREATE TABLE roles (
        id INTEGER PRIMARY KEY,
        employer_id INTEGER REFERENCES organizations(id),
        title TEXT NOT NULL,
        start_date TEXT,
        end_date TEXT,
        is_current INTEGER DEFAULT 0,
        work_arrangement TEXT,
        base_salary INTEGER,
        total_comp_notes TEXT,
        notes TEXT
      );

      CREATE TABLE education (
        id INTEGER PRIMARY KEY,
        type TEXT NOT NULL,
        institution TEXT,
        name TEXT NOT NULL,
        field TEXT,
        start_date TEXT,
        end_date TEXT,
        is_in_progress INTEGER DEFAULT 0,
        credential_id TEXT,
        expiration_date TEXT,
        url TEXT,
        notes TEXT
      );

      CREATE TABLE clearances (
        id INTEGER PRIMARY KEY,
        level TEXT NOT NULL,
        polygraph TEXT,
        status TEXT,
        sponsoring_agency TEXT,
        investigation_date TEXT,
        adjudication_date TEXT,
        reinvestigation_date TEXT,
        read_on TEXT,
        notes TEXT
      );

      CREATE TABLE skills (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        category TEXT,
        notes TEXT
      );

      CREATE TABLE languages (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        proficiency TEXT,
        reading_proficiency TEXT,
        writing_proficiency TEXT,
        certification TEXT,
        notes TEXT
      );

      CREATE TABLE bullets (
        id INTEGER PRIMARY KEY,
        content TEXT NOT NULL,
        framing TEXT,
        parent_id INTEGER,
        status TEXT DEFAULT 'accepted',
        notes TEXT
      );

      CREATE TABLE bullet_roles (
        bullet_id INTEGER,
        role_id INTEGER,
        is_primary INTEGER DEFAULT 0,
        PRIMARY KEY (bullet_id, role_id)
      );

      CREATE TABLE bullet_education (
        bullet_id INTEGER,
        education_id INTEGER,
        PRIMARY KEY (bullet_id, education_id)
      );

      CREATE TABLE bullet_skills (
        bullet_id INTEGER,
        skill_id INTEGER,
        PRIMARY KEY (bullet_id, skill_id)
      );

      CREATE TABLE resumes (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        target_company TEXT,
        target_role TEXT,
        notes TEXT
      );

      CREATE TABLE resume_bullets (
        resume_id INTEGER,
        bullet_id INTEGER,
        position INTEGER,
        section TEXT,
        PRIMARY KEY (resume_id, bullet_id)
      );
    `)

    // Insert sample data
    v1.exec(`
      INSERT INTO organizations VALUES (1, 'Acme Corp', 'company', 'Tech', 'large', 1, 'civilian', 'NYC', NULL, 'https://acme.com', NULL, NULL, NULL, NULL, NULL);
      INSERT INTO organizations VALUES (2, 'US Army', 'military', 'Defense', NULL, 1, 'military_active', 'Various', NULL, NULL, NULL, NULL, NULL, NULL, NULL);

      INSERT INTO roles VALUES (1, 1, 'Senior Engineer', '2022-01-01', NULL, 1, 'remote', 180000, NULL, NULL);
      INSERT INTO roles VALUES (2, 2, 'Signal Officer', '2018-06-01', '2022-01-01', 0, 'onsite', NULL, NULL, NULL);

      INSERT INTO education VALUES (1, 'degree', 'MIT', 'BS Computer Science', 'CS', '2014-09-01', '2018-05-01', 0, NULL, NULL, NULL, NULL);
      INSERT INTO education VALUES (2, 'certificate', 'AWS', 'Solutions Architect', 'Cloud', NULL, '2023-03-01', 0, 'AWS-SAA-123', '2026-03-01', 'https://aws.amazon.com', NULL);

      INSERT INTO clearances VALUES (1, 'TS/SCI', 'CI', 'active', 'DoD', '2019-01-01', '2019-06-01', '2024-01-01', 'SCI programs', 'Active clearance');

      INSERT INTO skills VALUES (1, 'Kubernetes', 'devops');
      INSERT INTO skills VALUES (2, 'Python', 'languages');
      INSERT INTO skills VALUES (3, 'AWS', 'cloud');

      INSERT INTO languages VALUES (1, 'English', 'native', NULL, NULL, NULL, NULL);
      INSERT INTO languages VALUES (2, 'Japanese', 'conversational', 'elementary', 'elementary', 'JLPT N3', NULL);

      INSERT INTO bullets VALUES (1, 'Led migration of cloud forensics platform to AWS.', 'devops', NULL, 'accepted', NULL);
      INSERT INTO bullets VALUES (2, 'Managed team of 4 engineers.', 'leadership', NULL, 'accepted', NULL);
      INSERT INTO bullets VALUES (3, 'Implemented CI/CD pipeline.', 'devops', 1, 'accepted', 'Sub-bullet of #1');

      INSERT INTO bullet_roles VALUES (1, 1, 1);
      INSERT INTO bullet_roles VALUES (2, 1, 1);
      INSERT INTO bullet_roles VALUES (3, 1, 0);
      INSERT INTO bullet_roles VALUES (1, 2, 0);

      INSERT INTO bullet_skills VALUES (1, 1);
      INSERT INTO bullet_skills VALUES (1, 3);
      INSERT INTO bullet_skills VALUES (2, 2);

      INSERT INTO resumes VALUES (1, 'AI Engineer Resume', 'Anthropic', 'AI Engineer', NULL);

      INSERT INTO resume_bullets VALUES (1, 1, 0, 'ai_engineering');
      INSERT INTO resume_bullets VALUES (1, 2, 1, 'leadership');
    `)

    // Create Forge test database with schema
    forgeDb = new Database(':memory:')
    // Apply Forge migrations (import the schema setup)
    // ... apply 001_initial.sql + 002_schema_evolution.sql ...
    // For testing, create minimal tables inline
    // (In actual test, use createTestDb() from core package)
  })

  afterEach(() => {
    v1.close()
    forgeDb.close()
  })

  test('imports all entity types correctly', () => {
    // Run import against both databases
    // ... call import functions ...

    // Verify organizations
    const orgs = forgeDb.query('SELECT * FROM organizations').all()
    expect(orgs.length).toBe(2)

    // Verify skills (3 + 2 languages = 5)
    const skills = forgeDb.query('SELECT * FROM skills').all()
    expect(skills.length).toBe(5)

    // Verify language skills have correct category
    const langSkills = forgeDb.query("SELECT * FROM skills WHERE category = 'language'").all()
    expect(langSkills.length).toBe(2)

    // Verify sources (2 roles + 2 education + 1 clearance = 5)
    const sources = forgeDb.query('SELECT * FROM sources').all()
    expect(sources.length).toBe(5)

    // Verify source_roles
    const sourceRoles = forgeDb.query('SELECT * FROM source_roles').all()
    expect(sourceRoles.length).toBe(2)

    // Verify bullets
    const bullets = forgeDb.query('SELECT * FROM bullets').all()
    expect(bullets.length).toBe(3)

    // Verify bullet_sources
    const bulletSources = forgeDb.query('SELECT * FROM bullet_sources').all()
    expect(bulletSources.length).toBe(4) // 3 bullet_roles + 0 bullet_education (none linked in test data)

    // Verify bullet_skills
    const bulletSkills = forgeDb.query('SELECT * FROM bullet_skills').all()
    expect(bulletSkills.length).toBe(3)

    // Verify resumes
    const resumes = forgeDb.query('SELECT * FROM resumes').all()
    expect(resumes.length).toBe(1)
    expect((resumes[0] as any).target_employer).toBe('Anthropic')

    // Verify synthetic perspectives (one per resume_bullet)
    const perspectives = forgeDb.query('SELECT * FROM perspectives').all()
    expect(perspectives.length).toBe(2)

    // Verify resume_entries (one per resume_bullet)
    const entries = forgeDb.query('SELECT * FROM resume_entries').all()
    expect(entries.length).toBe(2)
    expect((entries[0] as any).section).toBe('work_history')
    expect((entries[0] as any).content).toBeNull() // reference mode
  })

  test('import is idempotent — second run produces no duplicates', () => {
    // First run
    // ... call import functions ...

    const countBefore = forgeDb.query('SELECT COUNT(*) as c FROM v1_import_map').get() as any

    // Second run
    // ... call import functions again ...

    const countAfter = forgeDb.query('SELECT COUNT(*) as c FROM v1_import_map').get() as any
    expect(countAfter.c).toBe(countBefore.c)

    // Entity counts unchanged
    const orgs = forgeDb.query('SELECT COUNT(*) as c FROM organizations').get() as any
    expect(orgs.c).toBe(2) // still 2, not 4
  })

  test('bullet domain populated from v1 framing', () => {
    // ... run import ...

    const bullet = forgeDb.query("SELECT domain FROM bullets WHERE content LIKE '%cloud forensics%'").get() as any
    expect(bullet.domain).toBe('devops')
  })

  test('resume_entry notes preserve v1 section', () => {
    // ... run import ...

    const entry = forgeDb.query("SELECT notes FROM resume_entries WHERE position = 0").get() as any
    expect(entry.notes).toBe('v1_section: ai_engineering')
  })
})
```

**Acceptance Criteria:**
- [ ] Test creates a representative v1 database in memory
- [ ] Import runs successfully against the test data
- [ ] All entity counts match expected values
- [ ] UUID mappings are created in `v1_import_map`
- [ ] Second import run produces zero new rows
- [ ] Bullet domain is populated from v1 framing
- [ ] Resume entry notes preserve v1 section values
- [ ] Organization linkages are correct (role -> org via import map)
- [ ] Language proficiency info is in skill notes

---

## Task 13.7: Register All New Subcommands in CLI Index

**Goal:** Register all new CLI subcommands in `packages/cli/src/index.ts`. This task MUST run after T13.1-T13.5 are complete.

**File:** `packages/cli/src/index.ts`

**Acceptance Criteria:**
- [ ] All new commands imported: `orgCommand`, `noteCommand`, `skillCommand`, `importCommand`
- [ ] All new commands registered in `subCommands`: `org`, `note`, `skill`, `import`
- [ ] `forge --help` shows all subcommands
- [ ] Each subcommand is reachable: `forge org --help`, `forge note --help`, `forge skill --help`, `forge import --help`

After all tasks, the CLI index registers new commands:

```typescript
#!/usr/bin/env bun
import { defineCommand, runMain } from 'citty'
import { sourceCommand } from './commands/source'
import { bulletCommand } from './commands/bullet'
import { perspectiveCommand } from './commands/perspective'
import { resumeCommand } from './commands/resume'
import { reviewCommand } from './commands/review'
import { orgCommand } from './commands/organization'
import { noteCommand } from './commands/note'
import { skillCommand } from './commands/skill'
import { importCommand } from './commands/import'

const main = defineCommand({
  meta: {
    name: 'forge',
    version: '0.1.0',
    description: 'Forge — AI-powered resume builder CLI',
  },
  args: {
    json: {
      type: 'boolean',
      description: 'Output results as JSON for machine-readable consumption',
      default: false,
      global: true,
    },
  },
  subCommands: {
    source: sourceCommand,
    bullet: bulletCommand,
    perspective: perspectiveCommand,
    resume: resumeCommand,
    review: reviewCommand,
    org: orgCommand,        // NEW
    note: noteCommand,      // NEW
    skill: skillCommand,    // NEW
    import: importCommand,  // NEW
  },
})

runMain(main)
```

---

## Parallelization

```
Task 13.1 (source cmds) ────┐
Task 13.2 (org cmds)    ────┤  All CLI commands can run in parallel
Task 13.3 (note cmds)   ────┤  (each is independent)
Task 13.4 (skill cmds)  ────┘
                              │
Task 13.5 (import cmd)  ────► depends on schema evolution migration being applied
                              ► depends on SDK types (for type references)
                              ► but does NOT depend on 13.1-13.4
                              │
Task 13.6 (import test) ────► depends on 13.5
                              │
Task 13.7 (CLI index)   ────► depends on 13.1-13.5 (all command files must exist)
                              ► modifies shared file packages/cli/src/index.ts
```

Tasks 13.1-13.4 are independent CLI command groups that can all be developed in parallel. Task 13.5 (import) is independent of them but requires the schema to exist. Task 13.6 (test) depends only on 13.5. Task 13.7 (CLI index registration) must run last -- it modifies the shared `packages/cli/src/index.ts` file to register all new subcommands.
