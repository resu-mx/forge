/**
 * Strict `updated_at` parity across the 14 entities whose updates refresh it.
 *
 * TS sets `updated_at` in a beforeUpdate hook (storage/hooks/common.ts); Rust sets it inline
 * in each store. For each entity: create it through the API, write a fixed old `updated_at`
 * with SQL, update it through its main route, then read the stored value back. It must be
 * strictly later than the old one and have the canonical `YYYY-MM-DDTHH:MM:SSZ` form.
 * Seeding an old value (rather than comparing `>=` or sleeping) is what lets a missed bump fail.
 *
 * The cross-entity pin has one shared exception, also pinned here: a sources PATCH that only
 * touches extension fields (role, project, education, presentation) leaves `sources.updated_at`
 * alone in both runtimes.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'

const OLD = '2020-01-01T00:00:00Z'
const ISO_Z = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/

describe('updated_at is refreshed on update', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  async function create(method: string, route: string, body: unknown, expectedStatus: number): Promise<any> {
    const res = await apiRequest(ctx.app, method, route, body)
    expect(res.status).toBe(expectedStatus)
    return (await res.json()).data
  }

  function seedOld(table: string, id: string): void {
    ctx.db.run(`UPDATE ${table} SET updated_at = ? WHERE id = ?`, [OLD, id])
    // Read it back rather than trust `changes`: triggers on some tables inflate that count.
    expect(storedUpdatedAt(table, id)).toBe(OLD)
  }

  function storedUpdatedAt(table: string, id: string): string {
    const row = ctx.db.query(`SELECT updated_at FROM ${table} WHERE id = ?`).get(id) as { updated_at: string } | null
    expect(row).not.toBeNull()
    return row!.updated_at
  }

  async function newResume(): Promise<string> {
    return (await create('POST', '/resumes', { name: 'R', target_role: 'Eng', target_employer: 'Acme', archetype: 'agentic-ai' }, 201)).id
  }

  async function newSection(resumeId: string): Promise<string> {
    return (await create('POST', `/resumes/${resumeId}/sections`, { title: 'Experience', entry_type: 'experience', position: 0 }, 201)).id
  }

  // Each case creates the entity and returns its id plus the request that updates it.
  const cases: Array<{
    table: string
    setup: () => Promise<{ id: string; update: () => Promise<Response> }>
  }> = [
    {
      table: 'sources',
      setup: async () => {
        const { id } = await create('POST', '/sources', { title: 'Role', description: 'Did things.', source_type: 'role' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/sources/${id}`, { title: 'Renamed' }) }
      },
    },
    {
      table: 'resume_entries',
      setup: async () => {
        const resumeId = await newResume()
        const sectionId = await newSection(resumeId)
        const { id } = await create('POST', `/resumes/${resumeId}/entries`, { section_id: sectionId, content: 'Freeform' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/entries/${id}`, { position: 3 }) }
      },
    },
    {
      table: 'organizations',
      setup: async () => {
        const { id } = await create('POST', '/organizations', { name: 'Acme', org_type: 'company' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/organizations/${id}`, { name: 'Acme Corp' }) }
      },
    },
    {
      table: 'contacts',
      setup: async () => {
        const { id } = await create('POST', '/contacts', { name: 'Ada' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/contacts/${id}`, { name: 'Ada L.' }) }
      },
    },
    {
      table: 'resumes',
      setup: async () => {
        const id = await newResume()
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/resumes/${id}`, { name: 'Renamed' }) }
      },
    },
    {
      table: 'resume_sections',
      setup: async () => {
        const resumeId = await newResume()
        const id = await newSection(resumeId)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/resumes/${resumeId}/sections/${id}`, { title: 'Work History' }) }
      },
    },
    {
      table: 'summaries',
      setup: async () => {
        const { id } = await create('POST', '/summaries', { title: 'Summary', role: 'Engineer' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/summaries/${id}`, { title: 'Summary v2' }) }
      },
    },
    {
      table: 'job_descriptions',
      setup: async () => {
        // Migration 028 leaves an AFTER UPDATE trigger (jd_updated_at) that stamps `now` on every
        // job_descriptions update, which would mask a missed bump in the application code and
        // overwrite the seeded old value. Drop it on this throwaway test database so the check
        // exercises the runtime's own updated_at handling.
        ctx.db.run('DROP TRIGGER IF EXISTS jd_updated_at')
        const { id } = await create('POST', '/job-descriptions', { title: 'Engineer', raw_text: 'Build things.' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/job-descriptions/${id}`, { title: 'Senior Engineer' }) }
      },
    },
    {
      table: 'credentials',
      setup: async () => {
        const { id } = await create(
          'POST',
          '/credentials',
          {
            credential_type: 'clearance',
            label: 'Old',
            details: { level: 'top_secret', polygraph: 'ci', clearance_type: 'personnel', access_programs: ['sci'] },
          },
          201,
        )
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/credentials/${id}`, { label: 'New' }) }
      },
    },
    {
      table: 'certifications',
      setup: async () => {
        const { id } = await create('POST', '/certifications', { short_name: 'CKA', long_name: 'Certified Kubernetes Administrator' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/certifications/${id}`, { short_name: 'CKAD' }) }
      },
    },
    {
      table: 'user_profile',
      setup: async () => {
        const res = await apiRequest(ctx.app, 'GET', '/profile')
        expect(res.status).toBe(200)
        const { id } = (await res.json()).data
        return { id, update: () => apiRequest(ctx.app, 'PATCH', '/profile', { name: 'Adam' }) }
      },
    },
    {
      table: 'user_notes',
      setup: async () => {
        const { id } = await create('POST', '/notes', { content: 'A note' }, 201)
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/notes/${id}`, { content: 'An edited note' }) }
      },
    },
    {
      table: 'resume_templates',
      setup: async () => {
        const { id } = await create(
          'POST',
          '/templates',
          { name: 'Template', sections: [{ title: 'Experience', entry_type: 'experience', position: 0 }] },
          201,
        )
        return { id, update: () => apiRequest(ctx.app, 'PATCH', `/templates/${id}`, { name: 'Template v2' }) }
      },
    },
    {
      table: 'answer_bank',
      setup: async () => {
        const { id } = await create('PUT', '/profile/answers', { field_kind: 'work_authorization', label: 'Work Authorization', value: 'US Citizen' }, 200)
        // The upsert route is the update path: same field_kind, new value.
        return {
          id,
          update: () => apiRequest(ctx.app, 'PUT', '/profile/answers', { field_kind: 'work_authorization', label: 'Work Authorization', value: 'Visa' }),
        }
      },
    },
  ]

  for (const entity of cases) {
    test(`${entity.table}: update sets a strictly newer, canonical updated_at`, async () => {
      const { id, update } = await entity.setup()
      seedOld(entity.table, id)

      const res = await update()
      expect(res.status).toBe(200)

      const after = storedUpdatedAt(entity.table, id)
      expect(after).toMatch(ISO_Z)
      expect(after > OLD).toBe(true)
    })
  }

  // ── sources: extension-only PATCH does not bump the base row ───────

  describe('sources PATCH touching only extension fields', () => {
    const extensionCases: Array<{ name: string; create: Record<string, unknown>; patch: Record<string, unknown> }> = [
      {
        name: 'role',
        create: { title: 'Role', description: 'Did things.', source_type: 'role' },
        patch: { role: { work_arrangement: 'remote' } },
      },
      {
        name: 'project',
        create: { title: 'Project', description: 'Built things.', source_type: 'project' },
        patch: { project: { url: 'https://example.test/project' } },
      },
      {
        name: 'education',
        create: { title: 'Course', description: 'Learned things.', source_type: 'education', education: { education_type: 'course' } },
        patch: { education: { field: 'Physics' } },
      },
      {
        name: 'presentation',
        create: { title: 'Talk', description: 'Gave a talk.', source_type: 'presentation' },
        patch: { presentation: { venue: 'KubeCon' } },
      },
    ]

    for (const ext of extensionCases) {
      test(`${ext.name}: sources.updated_at is unchanged`, async () => {
        const { id } = await create('POST', '/sources', ext.create, 201)
        seedOld('sources', id)

        const res = await apiRequest(ctx.app, 'PATCH', `/sources/${id}`, ext.patch)
        expect(res.status).toBe(200)

        expect(storedUpdatedAt('sources', id)).toBe(OLD)
      })
    }
  })
})
