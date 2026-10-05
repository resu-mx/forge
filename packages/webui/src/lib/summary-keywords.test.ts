import { describe, expect, test } from 'bun:test'
import type { Result, Skill } from '@forge/sdk'
import { attachKeywords, keywordLoadFailureMessage, keywordSaveToast } from './summary-keywords'

const k8s = { id: 'k1', name: 'Kubernetes', category: 'tool' } as Skill
const tf = { id: 'k2', name: 'Terraform', category: 'tool' } as Skill
const ok: Result<void> = { ok: true, data: undefined }
const notPorted: Result<void> = {
  ok: false,
  error: {
    code: 'NOT_IMPLEMENTED',
    message: 'POST /api/summaries/s1/skills is not available in the browser runtime yet',
  },
}

describe('attachKeywords', () => {
  test('attaches in pick order, one at a time', async () => {
    const calls: string[] = []
    let inFlight = 0
    let maxInFlight = 0
    await attachKeywords('s1', [k8s, tf], async (_id, skillId) => {
      inFlight++
      maxInFlight = Math.max(maxInFlight, inFlight)
      calls.push(skillId)
      await new Promise((r) => setTimeout(r, 1))
      inFlight--
      return ok
    })
    expect(calls).toEqual(['k1', 'k2'])
    expect(maxInFlight).toBe(1)
  })

  test('returns the skills that failed', async () => {
    expect(
      await attachKeywords('s1', [k8s, tf], async (_id, skillId) => (skillId === 'k2' ? notPorted : ok)),
    ).toEqual([tf])
  })
})

describe('keywordSaveToast', () => {
  test('success when nothing failed', () =>
    expect(keywordSaveToast([])).toEqual({ type: 'success', message: 'Summary created' }))
  test('names one failure', () =>
    expect(keywordSaveToast([tf])).toEqual({
      type: 'error',
      message: 'Summary created, but 1 keyword was not saved: Terraform',
    }))
  test('names several', () =>
    expect(keywordSaveToast([k8s, tf]).message).toBe(
      'Summary created, but 2 keywords were not saved: Kubernetes, Terraform',
    ))
})

describe('keywordLoadFailureMessage', () => {
  test('null when nothing failed', () => expect(keywordLoadFailureMessage(0, 3)).toBeNull())
  test('counts failures and appends the reason', () =>
    expect(keywordLoadFailureMessage(3, 3, 'GET … is not available in the browser runtime yet')).toBe(
      "Couldn't load keywords for 3 of 3 summaries: GET … is not available in the browser runtime yet",
    ))
  test('singular total', () =>
    expect(keywordLoadFailureMessage(1, 1)).toBe("Couldn't load keywords for 1 of 1 summary"))
})
