/**
 * Note reference routes: the polymorphic `note_references` junction.
 *
 * The entity types are the ones the schema CHECK allows (migration 037 added
 * `credential` and `certification`), and both runtimes must accept all of them.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { testUuid } from '../../db/__tests__/helpers'

const ENTITY_TYPES = [
  'source',
  'bullet',
  'perspective',
  'resume_entry',
  'resume',
  'skill',
  'organization',
  'job_description',
  'contact',
  'credential',
  'certification',
]

describe('Note reference routes', () => {
  let ctx: TestContext

  beforeEach(() => {
    ctx = createTestApp()
  })

  afterEach(() => {
    ctx.db.close()
  })

  async function createNote(content = 'A note'): Promise<string> {
    const res = await apiRequest(ctx.app, 'POST', '/notes', { content })
    expect(res.status).toBe(201)
    return (await res.json()).data.id
  }

  test('accepts and lists every entity type the schema allows', async () => {
    const noteId = await createNote()
    const ids: Record<string, string> = {}

    for (const entityType of ENTITY_TYPES) {
      ids[entityType] = testUuid()
      const res = await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, {
        entity_type: entityType,
        entity_id: ids[entityType],
      })
      expect(res.status).toBe(201)
      expect(await res.json()).toEqual({ data: null })
    }

    const res = await apiRequest(ctx.app, 'GET', `/notes/${noteId}`)
    expect(res.status).toBe(200)
    const refs: Array<{ note_id: string; entity_type: string; entity_id: string }> = (await res.json()).data.references
    expect(refs).toHaveLength(ENTITY_TYPES.length)
    for (const entityType of ENTITY_TYPES) {
      const ref = refs.find((r) => r.entity_type === entityType)
      expect(ref).toEqual({ note_id: noteId, entity_type: entityType, entity_id: ids[entityType] })
    }
  })

  test('finds notes by entity for every entity type', async () => {
    const noteId = await createNote()
    for (const entityType of ENTITY_TYPES) {
      const entityId = testUuid()
      await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, { entity_type: entityType, entity_id: entityId })

      const res = await apiRequest(ctx.app, 'GET', `/notes/by-entity/${entityType}/${entityId}`)
      expect(res.status).toBe(200)
      const notes = (await res.json()).data
      expect(notes).toHaveLength(1)
      expect(notes[0].id).toBe(noteId)
    }
  })

  test('removes a credential and a certification reference', async () => {
    const noteId = await createNote()
    const credentialId = testUuid()
    const certificationId = testUuid()
    await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, { entity_type: 'credential', entity_id: credentialId })
    await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, { entity_type: 'certification', entity_id: certificationId })

    let res = await apiRequest(ctx.app, 'DELETE', `/notes/${noteId}/references/credential/${credentialId}`)
    expect(res.status).toBe(204)

    res = await apiRequest(ctx.app, 'GET', `/notes/${noteId}`)
    const refs = (await res.json()).data.references
    expect(refs).toEqual([{ note_id: noteId, entity_type: 'certification', entity_id: certificationId }])

    res = await apiRequest(ctx.app, 'DELETE', `/notes/${noteId}/references/certification/${certificationId}`)
    expect(res.status).toBe(204)
    res = await apiRequest(ctx.app, 'GET', `/notes/${noteId}`)
    expect((await res.json()).data.references).toEqual([])
  })

  test('400 for an unknown entity_type', async () => {
    const noteId = await createNote()
    const res = await apiRequest(ctx.app, 'POST', `/notes/${noteId}/references`, {
      entity_type: 'bogus',
      entity_id: testUuid(),
    })
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
    expect(body.error.message).toContain('Invalid entity_type')
  })

  test('400 for an unknown entity_type in a by-entity lookup', async () => {
    const res = await apiRequest(ctx.app, 'GET', `/notes/by-entity/bogus/${testUuid()}`)
    expect(res.status).toBe(400)
    expect((await res.json()).error.code).toBe('VALIDATION_ERROR')
  })

  test('404 when removing a reference that does not exist', async () => {
    const noteId = await createNote()
    for (const entityType of ['credential', 'bogus']) {
      const res = await apiRequest(ctx.app, 'DELETE', `/notes/${noteId}/references/${entityType}/${testUuid()}`)
      expect(res.status).toBe(404)
      expect((await res.json()).error.code).toBe('NOT_FOUND')
    }
  })

  test('404 when the note does not exist', async () => {
    const res = await apiRequest(ctx.app, 'POST', `/notes/${testUuid()}/references`, {
      entity_type: 'certification',
      entity_id: testUuid(),
    })
    expect(res.status).toBe(404)
    expect((await res.json()).error.code).toBe('NOT_FOUND')
  })
})
