import { describe, expect, test } from 'bun:test'
import { cyrb128, rng } from '../prng'
import { Clock, isoZ, monthsAgoDate, parseAsOf } from '../time'

describe('prng', () => {
  test('cyrb128 is a pure function of its input', () => {
    expect(cyrb128('abc')).toEqual(cyrb128('abc'))
    expect(cyrb128('abc')).not.toEqual(cyrb128('abd'))
  })

  test('the same (seed, persona, stream) gives the same sequence', () => {
    const a = rng('s', 'p', 'x')
    const b = rng('s', 'p', 'x')
    expect(Array.from({ length: 20 }, () => a.next())).toEqual(Array.from({ length: 20 }, () => b.next()))
  })

  test('streams, personas and seeds are independent', () => {
    const first = (seed: string, persona: string, stream: string) => rng(seed, persona, stream).next()
    const base = first('s', 'p', 'x')
    expect(first('s', 'p', 'y')).not.toBe(base)
    expect(first('s', 'q', 'x')).not.toBe(base)
    expect(first('t', 'p', 'x')).not.toBe(base)
  })

  test('golden values (a change here changes every dataset fingerprint)', () => {
    const r = rng('forge-demo-v1', 'early-career-developer', 'golden')
    const values = Array.from({ length: 4 }, () => r.int(0, 1_000_000))
    expect(values).toEqual([910296, 368643, 257357, 59710])
  })

  test('next() is in [0, 1); int() and pick() stay in range', () => {
    const r = rng('s', 'p', 'range')
    for (let i = 0; i < 2000; i++) {
      const x = r.next()
      expect(x >= 0 && x < 1).toBe(true)
      const n = r.int(3, 7)
      expect(Number.isInteger(n) && n >= 3 && n <= 7).toBe(true)
      expect(['a', 'b', 'c']).toContain(r.pick(['a', 'b', 'c']))
    }
    expect(() => r.int(5, 4)).toThrow()
    expect(() => r.pick([])).toThrow()
  })
})

describe('time', () => {
  const asOf = parseAsOf('2026-09-30T17:00:00Z')

  test('parseAsOf accepts only YYYY-MM-DDTHH:MM:SSZ', () => {
    expect(() => parseAsOf('2026-09-30')).toThrow()
    expect(() => parseAsOf('2026-09-30T17:00:00.000Z')).toThrow()
    expect(isoZ(asOf)).toBe('2026-09-30T17:00:00Z')
  })

  test('monthsAgoDate', () => {
    expect(monthsAgoDate(asOf, 0)).toBe('2026-09-01')
    expect(monthsAgoDate(asOf, 13, 15)).toBe('2025-08-15')
    expect(monthsAgoDate(asOf, -18, 14)).toBe('2028-03-14')
  })

  test('Clock.at is deterministic, never after as_of, and orders children after parents', () => {
    const clock = new Clock(asOf, 'seed', 'persona')
    expect(clock.at(10, 'x')).toBe(new Clock(asOf, 'seed', 'persona').at(10, 'x'))
    expect(clock.at(0, 'today') < '2026-09-30T17:00:00Z').toBe(true)
    const parent = clock.at(5, 'parent')
    expect(clock.at(9, 'child', parent) > parent).toBe(true)
    expect(() => clock.at(-1, 'bad')).toThrow()
  })
})
