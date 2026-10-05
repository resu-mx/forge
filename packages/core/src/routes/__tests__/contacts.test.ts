/**
 * Contact relationship routes: link/unlink/list for organizations, job descriptions and
 * resumes, plus the reverse lookups. Runs in-process and, under `just parity`, against
 * forge-server.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import type { Database } from 'bun:sqlite'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedOrganization, seedJobDescription, seedResume } from '../../db/__tests__/helpers'

const MISSING = '00000000-0000-0000-0000-000000000000'

function seedContact(
  db: Database,
  name: string,
  extra: { title?: string; email?: string } = {},
): string {
  const id = crypto.randomUUID()
  db.run('INSERT INTO contacts (id, name, title, email) VALUES (?, ?, ?, ?)', [
    id,
    name,
    extra.title ?? null,
    extra.email ?? null,
  ])
  return id
}

describe('Contact relationship routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  async function listJson(path: string): Promise<{ status: number; data: unknown[] }> {
    const res = await apiRequest(ctx.app, 'GET', path)
    const body = await res.json()
    return { status: res.status, data: body.data }
  }

  describe('organizations', () => {
    test('POST links with 201 and an empty body; GET lists it', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })

      const res = await apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
        organization_id: o,
        relationship: 'recruiter',
      })
      expect(res.status).toBe(201)
      expect(await res.text()).toBe('')

      const list = await listJson(`/contacts/${c}/organizations`)
      expect(list.status).toBe(200)
      expect(list.data).toEqual([{ id: o, name: 'Acme', relationship: 'recruiter' }])
    })

    test('POST with an invalid relationship returns 400 naming the valid values', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })
      const res = await apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
        organization_id: o,
        relationship: 'boss',
      })
      expect(res.status).toBe(400)
      const body = await res.json()
      expect(body.error.code).toBe('VALIDATION_ERROR')
      expect(body.error.message).toContain('recruiter, hr, referral, peer, manager, other')
    })

    test('POST with no relationship returns 400', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })
      const res = await apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
        organization_id: o,
      })
      expect(res.status).toBe(400)
      expect((await res.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('linking twice keeps one row; two relationships on one pair are both listed', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })
      const link = (relationship: string) =>
        apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
          organization_id: o,
          relationship,
        })

      expect((await link('recruiter')).status).toBe(201)
      expect((await link('recruiter')).status).toBe(201)
      expect((await listJson(`/contacts/${c}/organizations`)).data).toHaveLength(1)

      expect((await link('peer')).status).toBe(201)
      const rels = (
        (await listJson(`/contacts/${c}/organizations`)).data as { relationship: string }[]
      )
        .map(r => r.relationship)
        .sort()
      expect(rels).toEqual(['peer', 'recruiter'])
    })

    test('POST without organization_id returns 400', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const res = await apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
        relationship: 'recruiter',
      })
      expect(res.status).toBe(400)
      expect((await res.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('POST for an unknown organization or contact returns 400 VALIDATION_ERROR', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })

      const noOrg = await apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
        organization_id: MISSING,
        relationship: 'hr',
      })
      expect(noOrg.status).toBe(400)
      expect((await noOrg.json()).error.code).toBe('VALIDATION_ERROR')

      const noContact = await apiRequest(ctx.app, 'POST', `/contacts/${MISSING}/organizations`, {
        organization_id: o,
        relationship: 'hr',
      })
      expect(noContact.status).toBe(400)
      expect((await noContact.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('DELETE unlinks with 204, is idempotent, and rejects an invalid relationship', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })
      await apiRequest(ctx.app, 'POST', `/contacts/${c}/organizations`, {
        organization_id: o,
        relationship: 'recruiter',
      })

      const first = await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/organizations/${o}/recruiter`)
      expect(first.status).toBe(204)
      expect(await first.text()).toBe('')
      expect((await listJson(`/contacts/${c}/organizations`)).data).toEqual([])

      const again = await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/organizations/${o}/recruiter`)
      expect(again.status).toBe(204)

      const bad = await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/organizations/${o}/boss`)
      expect(bad.status).toBe(400)
    })

    test('GET for an unknown contact returns 200 []', async () => {
      const list = await listJson(`/contacts/${MISSING}/organizations`)
      expect(list.status).toBe(200)
      expect(list.data).toEqual([])
    })
  })

  describe('job descriptions', () => {
    test('POST links with 201; GET lists organization_name (null without an organization)', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })
      const withOrg = seedJobDescription(ctx.db, { title: 'Platform SRE', organizationId: o })
      const bare = seedJobDescription(ctx.db, { title: 'Another role' })

      const res = await apiRequest(ctx.app, 'POST', `/contacts/${c}/job-descriptions`, {
        job_description_id: withOrg,
        relationship: 'hiring_manager',
      })
      expect(res.status).toBe(201)
      expect(await res.text()).toBe('')
      await apiRequest(ctx.app, 'POST', `/contacts/${c}/job-descriptions`, {
        job_description_id: bare,
        relationship: 'other',
      })

      const list = await listJson(`/contacts/${c}/job-descriptions`)
      expect(list.data).toEqual([
        { id: bare, title: 'Another role', organization_name: null, relationship: 'other' },
        {
          id: withOrg,
          title: 'Platform SRE',
          organization_name: 'Acme',
          relationship: 'hiring_manager',
        },
      ])
    })

    test('POST validates the relationship and the target', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const j = seedJobDescription(ctx.db, { title: 'Platform SRE' })

      const bad = await apiRequest(ctx.app, 'POST', `/contacts/${c}/job-descriptions`, {
        job_description_id: j,
        relationship: 'boss',
      })
      expect(bad.status).toBe(400)
      expect((await bad.json()).error.message).toContain(
        'hiring_manager, recruiter, interviewer, referral, other',
      )

      const noId = await apiRequest(ctx.app, 'POST', `/contacts/${c}/job-descriptions`, {
        relationship: 'recruiter',
      })
      expect(noId.status).toBe(400)

      const unknown = await apiRequest(ctx.app, 'POST', `/contacts/${c}/job-descriptions`, {
        job_description_id: MISSING,
        relationship: 'recruiter',
      })
      expect(unknown.status).toBe(400)
      expect((await unknown.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('DELETE unlinks with 204 and rejects an invalid relationship', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const j = seedJobDescription(ctx.db, { title: 'Platform SRE' })
      await apiRequest(ctx.app, 'POST', `/contacts/${c}/job-descriptions`, {
        job_description_id: j,
        relationship: 'interviewer',
      })

      const del = await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/job-descriptions/${j}/interviewer`)
      expect(del.status).toBe(204)
      expect((await listJson(`/contacts/${c}/job-descriptions`)).data).toEqual([])
      expect(
        (await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/job-descriptions/${j}/interviewer`)).status,
      ).toBe(204)
      expect(
        (await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/job-descriptions/${j}/boss`)).status,
      ).toBe(400)
    })

    test('GET for an unknown contact returns 200 []', async () => {
      expect((await listJson(`/contacts/${MISSING}/job-descriptions`)).data).toEqual([])
    })
  })

  describe('resumes', () => {
    test('POST links with 201; GET lists it; DELETE unlinks', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const rs = seedResume(ctx.db, { name: 'Platform resume' })

      const res = await apiRequest(ctx.app, 'POST', `/contacts/${c}/resumes`, {
        resume_id: rs,
        relationship: 'reference',
      })
      expect(res.status).toBe(201)
      expect(await res.text()).toBe('')

      expect((await listJson(`/contacts/${c}/resumes`)).data).toEqual([
        { id: rs, name: 'Platform resume', relationship: 'reference' },
      ])

      const del = await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/resumes/${rs}/reference`)
      expect(del.status).toBe(204)
      expect((await listJson(`/contacts/${c}/resumes`)).data).toEqual([])
      expect(
        (await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/resumes/${rs}/reference`)).status,
      ).toBe(204)
    })

    test('POST validates the relationship and the target', async () => {
      const c = seedContact(ctx.db, 'Ada')
      const rs = seedResume(ctx.db, { name: 'Platform resume' })

      const bad = await apiRequest(ctx.app, 'POST', `/contacts/${c}/resumes`, {
        resume_id: rs,
        relationship: 'hr',
      })
      expect(bad.status).toBe(400)
      expect((await bad.json()).error.message).toContain('reference, recommender, other')

      expect(
        (await apiRequest(ctx.app, 'POST', `/contacts/${c}/resumes`, { relationship: 'reference' }))
          .status,
      ).toBe(400)

      const unknown = await apiRequest(ctx.app, 'POST', `/contacts/${c}/resumes`, {
        resume_id: MISSING,
        relationship: 'reference',
      })
      expect(unknown.status).toBe(400)
      expect((await unknown.json()).error.code).toBe('VALIDATION_ERROR')

      expect(
        (await apiRequest(ctx.app, 'DELETE', `/contacts/${c}/resumes/${rs}/boss`)).status,
      ).toBe(400)
    })

    test('GET for an unknown contact returns 200 []', async () => {
      expect((await listJson(`/contacts/${MISSING}/resumes`)).data).toEqual([])
    })
  })

  describe('reverse lookups', () => {
    test('return contact links sorted by contact name', async () => {
      // Same-case names so binary (SQLite) and localeCompare orderings agree.
      const grace = seedContact(ctx.db, 'Grace', { title: 'VP', email: 'grace@example.com' })
      const ada = seedContact(ctx.db, 'Ada')
      const o = seedOrganization(ctx.db, { name: 'Acme' })
      const j = seedJobDescription(ctx.db, { title: 'Platform SRE', organizationId: o })
      const rs = seedResume(ctx.db, { name: 'Platform resume' })

      for (const [who, rel] of [
        [grace, 'manager'],
        [ada, 'recruiter'],
      ] as const) {
        await apiRequest(ctx.app, 'POST', `/contacts/${who}/organizations`, {
          organization_id: o,
          relationship: rel,
        })
        await apiRequest(ctx.app, 'POST', `/contacts/${who}/job-descriptions`, {
          job_description_id: j,
          relationship: 'other',
        })
        await apiRequest(ctx.app, 'POST', `/contacts/${who}/resumes`, {
          resume_id: rs,
          relationship: 'other',
        })
      }

      const byOrg = await listJson(`/organizations/${o}/contacts`)
      expect(byOrg.status).toBe(200)
      expect(byOrg.data).toEqual([
        {
          contact_id: ada,
          contact_name: 'Ada',
          contact_title: null,
          contact_email: null,
          relationship: 'recruiter',
        },
        {
          contact_id: grace,
          contact_name: 'Grace',
          contact_title: 'VP',
          contact_email: 'grace@example.com',
          relationship: 'manager',
        },
      ])

      for (const path of [`/job-descriptions/${j}/contacts`, `/resumes/${rs}/contacts`]) {
        const rows = (await listJson(path)).data as { contact_name: string; relationship: string }[]
        expect(rows.map(r => r.contact_name)).toEqual(['Ada', 'Grace'])
        expect(rows[0]!.relationship).toBe('other')
      }
    })

    test('return 200 [] for an unknown id', async () => {
      for (const path of [
        `/organizations/${MISSING}/contacts`,
        `/job-descriptions/${MISSING}/contacts`,
        `/resumes/${MISSING}/contacts`,
      ]) {
        const list = await listJson(path)
        expect(list.status).toBe(200)
        expect(list.data).toEqual([])
      }
    })
  })
})
