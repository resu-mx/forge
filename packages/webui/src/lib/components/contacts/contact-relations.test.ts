import { describe, expect, test } from 'bun:test'
import {
  linkRelation,
  listRelations,
  listTargets,
  unlinkRelation,
  type RelationClient,
} from './contact-relations'

const ERR = { ok: false, error: { code: 'NETWORK_ERROR', message: 'boom' } }

function fake() {
  const calls: unknown[][] = []
  const rec = (name: string, result: unknown) => async (...args: unknown[]) => {
    calls.push([name, ...args])
    return result
  }
  const ok = { ok: true, data: undefined }
  const client = {
    contacts: {
      linkOrganization: rec('linkOrganization', ok),
      linkJobDescription: rec('linkJobDescription', ok),
      linkResume: rec('linkResume', ok),
      unlinkOrganization: rec('unlinkOrganization', ok),
      unlinkJobDescription: rec('unlinkJobDescription', ok),
      unlinkResume: rec('unlinkResume', ok),
      listOrganizations: rec('listOrganizations', {
        ok: true,
        data: [{ id: 'o1', name: 'Acme', relationship: 'hr' }],
      }),
      listJobDescriptions: rec('listJobDescriptions', {
        ok: true,
        data: [{ id: 'j1', title: 'Staff Engineer', organization_name: 'Acme', relationship: 'recruiter' }],
      }),
      listResumes: rec('listResumes', { ok: true, data: [{ id: 'r1', name: 'Main', relationship: 'reference' }] }),
      list: () => {
        throw new Error('the organization picker must not list contacts')
      },
    },
    organizations: {
      list: rec('organizations.list', {
        ok: true,
        data: [
          { id: 'o2', name: 'Zed' },
          { id: 'o1', name: 'Acme' },
        ],
        pagination: {},
      }),
    },
    jobDescriptions: {
      list: rec('jobDescriptions.list', {
        ok: true,
        data: [
          { id: 'j1', title: 'Staff', organization_name: 'Acme' },
          { id: 'j2', title: 'Alpha', organization_name: null },
        ],
        pagination: {},
      }),
    },
    resumes: {
      list: rec('resumes.list', { ok: true, data: [{ id: 'r1', name: 'Main' }], pagination: {} }),
    },
  } as unknown as RelationClient
  return { client, calls }
}

describe('contact-relations', () => {
  test('link puts the contact first and the picked target second, for every kind', async () => {
    const { client, calls } = fake()
    await linkRelation(client, 'organization', 'c1', 'o1', 'recruiter')
    await linkRelation(client, 'job_description', 'c1', 'j1', 'interviewer')
    await linkRelation(client, 'resume', 'c1', 'r1', 'reference')
    expect(calls).toEqual([
      ['linkOrganization', 'c1', 'o1', 'recruiter'],
      ['linkJobDescription', 'c1', 'j1', 'interviewer'],
      ['linkResume', 'c1', 'r1', 'reference'],
    ])
  })

  test('unlink sends the target id, not the contact id twice, for every kind', async () => {
    const { client, calls } = fake()
    await unlinkRelation(client, 'organization', 'c1', 'o1', 'hr')
    await unlinkRelation(client, 'job_description', 'c1', 'j1', 'recruiter')
    await unlinkRelation(client, 'resume', 'c1', 'r1', 'reference')
    expect(calls).toEqual([
      ['unlinkOrganization', 'c1', 'o1', 'hr'],
      ['unlinkJobDescription', 'c1', 'j1', 'recruiter'],
      ['unlinkResume', 'c1', 'r1', 'reference'],
    ])
  })

  test('rows keep the target id and sublabel', async () => {
    const { client } = fake()
    expect(await listRelations(client, 'job_description', 'c1')).toEqual({
      ok: true,
      data: [{ target_id: 'j1', label: 'Staff Engineer', sublabel: 'Acme', relationship: 'recruiter' }],
    })
    expect(await listRelations(client, 'organization', 'c1')).toEqual({
      ok: true,
      data: [{ target_id: 'o1', label: 'Acme', sublabel: null, relationship: 'hr' }],
    })
    expect(await listRelations(client, 'resume', 'c1')).toEqual({
      ok: true,
      data: [{ target_id: 'r1', label: 'Main', sublabel: null, relationship: 'reference' }],
    })
  })

  test('a failed list comes back as { ok: false }', async () => {
    const { client } = fake()
    ;(client.contacts as any).listOrganizations = async () => ERR
    ;(client.organizations as any).list = async () => ERR
    expect(await listRelations(client, 'organization', 'c1')).toEqual(ERR as any)
    expect(await listTargets(client, 'organization')).toEqual(ERR as any)
  })

  test('the organization picker lists organizations (sorted, limit 200), never contacts', async () => {
    const { client, calls } = fake()
    expect(await listTargets(client, 'organization')).toEqual({
      ok: true,
      data: [
        { id: 'o1', label: 'Acme' },
        { id: 'o2', label: 'Zed' },
      ],
    })
    expect(calls).toEqual([['organizations.list', { limit: 200 }]])
  })

  test('the JD and resume pickers list their own entity', async () => {
    const { client, calls } = fake()
    expect(await listTargets(client, 'job_description')).toEqual({
      ok: true,
      data: [
        { id: 'j2', label: 'Alpha' },
        { id: 'j1', label: 'Staff — Acme' },
      ],
    })
    expect(await listTargets(client, 'resume')).toEqual({ ok: true, data: [{ id: 'r1', label: 'Main' }] })
    expect(calls).toEqual([
      ['jobDescriptions.list', { limit: 200 }],
      ['resumes.list', { limit: 200 }],
    ])
  })
})
