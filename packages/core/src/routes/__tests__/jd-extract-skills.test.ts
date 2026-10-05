/**
 * Tests for POST /job-descriptions/:id/extract-skills.
 * Pins the extraction context payload so both runtimes (Hono and the Rust server) agree,
 * including byte equality of prompt_template.
 */

import { describe, test, expect, beforeEach, afterEach } from 'bun:test'
import { createTestApp, apiRequest, type TestContext } from './helpers'
import { seedJobDescription, seedSkill } from '../../db/__tests__/helpers'
import { renderJDSkillExtractionPrompt } from '../../ai'

const INSTRUCTIONS =
  'Execute the prompt_template to extract skills from the JD text. For each extracted skill, check existing_skills for a match by name before creating new ones. Call forge_tag_jd_skill (or POST /api/job-descriptions/:id/skills) for each accepted skill.'

describe('POST /job-descriptions/:id/extract-skills', () => {
  let ctx: TestContext
  beforeEach(() => {
    ctx = createTestApp()
  })
  afterEach(() => {
    ctx.db.close()
  })

  test('returns the context payload with a single-string prompt_template', async () => {
    seedSkill(ctx.db, { name: 'Zzlang', category: 'language' })
    seedSkill(ctx.db, { name: 'Zzframe', category: 'framework' })
    const raw = 'We write Rust and ship with Docker.'
    const id = seedJobDescription(ctx.db, { rawText: raw })

    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${id}/extract-skills`)
    expect(res.status).toBe(200)
    const { data } = await res.json()
    expect(data.jd_raw_text).toBe(raw)
    expect(data.prompt_template).toBe(renderJDSkillExtractionPrompt(raw)) // byte equality across runtimes
    expect(data.instructions).toBe(INSTRUCTIONS)

    // Migration 041 seeds skills, so compare with the database rather than a fixed list.
    const expected = (
      ctx.db
        .query("SELECT id FROM skills WHERE category IN ('language', 'tool', 'other', 'soft_skill')")
        .all() as { id: string }[]
    )
      .map((r) => r.id)
      .sort()
    expect(data.existing_skills.map((s: { id: string }) => s.id).sort()).toEqual(expected)
  })

  test('each existing_skills row has exactly id, name and category', async () => {
    seedSkill(ctx.db, { name: 'Zzlang', category: 'language' })
    const id = seedJobDescription(ctx.db, { rawText: 'We write Rust.' })
    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${id}/extract-skills`)
    expect(res.status).toBe(200)
    const { data } = await res.json()
    expect(data.existing_skills.length).toBeGreaterThan(0)
    for (const s of data.existing_skills) {
      expect(Object.keys(s).sort()).toEqual(['category', 'id', 'name'])
    }
  })

  test('returns 404 NOT_FOUND for an unknown id', async () => {
    const res = await apiRequest(ctx.app, 'POST', '/job-descriptions/does-not-exist/extract-skills')
    expect(res.status).toBe(404)
    const body = await res.json()
    expect(body.error.code).toBe('NOT_FOUND')
  })

  test('returns 400 VALIDATION_ERROR for blank raw_text', async () => {
    const id = seedJobDescription(ctx.db, { rawText: '   ' })
    const res = await apiRequest(ctx.app, 'POST', `/job-descriptions/${id}/extract-skills`)
    expect(res.status).toBe(400)
    const body = await res.json()
    expect(body.error.code).toBe('VALIDATION_ERROR')
  })
})
