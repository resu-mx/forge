import { describe, expect, test } from 'bun:test'
import { DEMO_NS, isCanonicalUuid, personaUuid } from '../ids'
import { datasetKey } from '../postpass/manifest'

describe('ids', () => {
  test('Bun.randomUUIDv5 matches the RFC 4122 test vector', () => {
    expect(Bun.randomUUIDv5('python.org', 'dns')).toBe('886313e1-3b8a-5372-9b90-0c9aee199e5d')
  })

  test('the demo namespace is uuidv5("demo-data.resu.mx", DNS)', () => {
    expect(DEMO_NS).toBe('68c6c6a9-5066-5107-83ee-2a933d6565d4')
  })

  test('persona uuids are stable (they are R2 keys: never change them)', () => {
    expect(personaUuid('early-career-developer')).toBe('5d660672-665c-512b-a0be-0b6a31d99e17')
    expect(personaUuid('cleared-security-engineer')).toBe('f3b8e198-e254-5acb-b6c7-a021fda8e3e4')
    expect(personaUuid('ai-ml-engineer')).toBe('c891cd67-1e4e-5908-9aa5-1f5eee23be28')
  })

  test('persona uuids are canonical version-5 uuids', () => {
    const id = personaUuid('early-career-developer')
    expect(isCanonicalUuid(id)).toBe(true)
    expect(id[14]).toBe('5')
    expect(isCanonicalUuid(id.toUpperCase())).toBe(false)
    expect(isCanonicalUuid(`${id}x`)).toBe(false)
  })

  test('dataset key layout', () => {
    expect(datasetKey(personaUuid('early-career-developer'))).toBe('user/5d660672-665c-512b-a0be-0b6a31d99e17/data.sqlite')
  })
})
