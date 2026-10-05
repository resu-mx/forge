import { describe, expect, test } from 'bun:test'
import type { Result, Skill } from '@forge/sdk'
import { acceptFailureMessage, acceptSkills } from '../extraction-accept'

const skill = (name: string) => ({ id: name.toLowerCase(), name, category: 'other' }) as Skill
const notPorted = {
  code: 'NOT_IMPLEMENTED',
  message: 'POST /api/job-descriptions/j1/skills is not available in the browser runtime yet',
}

describe('acceptSkills', () => {
  test('adds one at a time, in order', async () => {
    let inFlight = 0
    let maxInFlight = 0
    const seen: string[] = []
    const add = async (name: string): Promise<Result<Skill>> => {
      inFlight++
      maxInFlight = Math.max(maxInFlight, inFlight)
      seen.push(name)
      await new Promise((r) => setTimeout(r, 1))
      inFlight--
      return { ok: true, data: skill(name) }
    }
    const out = await acceptSkills(
      [
        { key: 'k8s', name: 'Kubernetes' },
        { key: 'tf', name: 'Terraform' },
      ],
      add,
    )
    expect(seen).toEqual(['Kubernetes', 'Terraform'])
    expect(maxInFlight).toBe(1)
    expect(out).toEqual({ accepted: ['k8s', 'tf'], failed: [] })
  })

  test('collects failures and keeps going', async () => {
    const add = async (name: string): Promise<Result<Skill>> =>
      name === 'Terraform' ? { ok: false, error: notPorted } : { ok: true, data: skill(name) }
    const out = await acceptSkills(
      [
        { key: 'tf', name: 'Terraform' },
        { key: 'k8s', name: 'Kubernetes' },
      ],
      add,
    )
    expect(out.accepted).toEqual(['k8s'])
    expect(out.failed).toEqual([{ key: 'tf', name: 'Terraform', message: notPorted.message }])
  })

  test('uses the describe callback for failure messages', async () => {
    const add = async (): Promise<Result<Skill>> => ({ ok: false, error: notPorted })
    const out = await acceptSkills([{ key: 'tf', name: 'Terraform' }], add, (e) => `friendly: ${e.code}`)
    expect(out.failed[0].message).toBe('friendly: NOT_IMPLEMENTED')
  })
})

describe('acceptFailureMessage', () => {
  test('null when nothing failed', () => expect(acceptFailureMessage([])).toBeNull())
  test('names a single failure with its reason', () =>
    expect(acceptFailureMessage([{ key: 'tf', name: 'Terraform', message: 'boom' }])).toBe(
      'Couldn\'t add "Terraform": boom',
    ))
  test('lists several failures by name', () =>
    expect(
      acceptFailureMessage([
        { key: 'a', name: 'A', message: 'x' },
        { key: 'b', name: 'B', message: 'y' },
      ]),
    ).toBe("Couldn't add 2 skills: A, B"))
})
