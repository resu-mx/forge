import { describe, expect, test } from 'bun:test'
import { entityCounts, hasLocalData, type CountableClient } from './local-data'

const total = (n: number) => async () => n
const boom = async (): Promise<number> => {
  throw new Error('runtime not ready')
}

describe('hasLocalData', () => {
  test('false when every count is zero', async () => {
    expect(await hasLocalData([total(0), total(0), total(0)])).toBe(false)
  })

  test('true as soon as any count is above zero, without asking the rest', async () => {
    let asked = 0
    const counted = (n: number) => async () => {
      asked++
      return n
    }
    expect(await hasLocalData([counted(0), counted(3), counted(9)])).toBe(true)
    expect(asked).toBe(2)
  })

  test('an unreadable count with nothing found elsewhere counts as data: when in doubt, keep it', async () => {
    expect(await hasLocalData([total(0), boom])).toBe(true)
  })

  test('an unreadable count does not hide data that was found', async () => {
    expect(await hasLocalData([boom, total(2)])).toBe(true)
  })

  test('no counts at all is no data', async () => {
    expect(await hasLocalData([])).toBe(false)
  })
})

describe('entityCounts', () => {
  const listable = (reply: unknown) => {
    const calls: Array<{ limit: number }> = []
    return {
      calls,
      list: async (params: { limit: number }) => {
        calls.push(params)
        return reply as { ok: true; pagination: { total: number } } | { ok: false }
      },
    }
  }

  function client(totals: [number, number, number, number]) {
    const [s, b, r, j] = totals.map((t) => listable({ ok: true, pagination: { total: t, offset: 0, limit: 1 } }))
    return { sources: s, bullets: b, resumes: r, jobDescriptions: j } satisfies CountableClient
  }

  test('asks each user-data resource for one row and reads the total', async () => {
    const c = client([0, 0, 4, 0])
    const counts = entityCounts(c)
    expect(counts).toHaveLength(4)
    expect(await Promise.all(counts.map((count) => count()))).toEqual([0, 0, 4, 0])
    for (const resource of Object.values(c)) expect(resource.calls).toEqual([{ limit: 1 }])
  })

  test('an API error is an unreadable count, which hasLocalData treats as data', async () => {
    const c = {
      ...client([0, 0, 0, 0]),
      sources: listable({ ok: false }),
    }
    expect(await hasLocalData(entityCounts(c))).toBe(true)
  })

  test('an empty database reads as no data', async () => {
    expect(await hasLocalData(entityCounts(client([0, 0, 0, 0])))).toBe(false)
  })
})
