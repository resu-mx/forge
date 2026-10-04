import { describe, expect, test } from 'bun:test'
import {
  createLocationWithAddress,
  listLocationsWithAddress,
  locationLabel,
  updateLocationWithAddress,
  type LocationClient,
  type LocationWithAddress,
} from './org-locations'

/** A fake client that records calls and keeps rows in memory. */
function fake(opts: { failGet?: boolean; failList?: boolean } = {}) {
  const calls: string[] = []
  const addresses = new Map<string, Record<string, unknown>>()
  const locations: Record<string, unknown>[] = []
  let n = 0
  const client = {
    addresses: {
      async create(input: Record<string, unknown>) {
        calls.push(`addresses.create ${JSON.stringify(input)}`)
        const row = { id: `a${++n}`, street_1: null, street_2: null, city: null, state: null, zip: null, country_code: 'US', ...input }
        addresses.set(row.id, row)
        return { ok: true, data: row }
      },
      async get(id: string) {
        const row = addresses.get(id)
        return row && !opts.failGet
          ? { ok: true, data: row }
          : { ok: false, error: { code: 'NOT_FOUND', message: `Address ${id} not found` } }
      },
      async update(id: string, input: Record<string, unknown>) {
        calls.push(`addresses.update ${id}`)
        const row = { ...addresses.get(id), ...input }
        addresses.set(id, row)
        return { ok: true, data: row }
      },
    },
    organizations: {
      async listLocations(orgId: string) {
        if (opts.failList) return { ok: false, error: { code: 'INTERNAL_ERROR', message: 'boom' } }
        return { ok: true, data: locations.filter((l) => l.organization_id === orgId) }
      },
      async createLocation(orgId: string, input: Record<string, unknown>) {
        calls.push(`createLocation ${JSON.stringify(input)}`)
        const row = { id: `l${++n}`, organization_id: orgId, modality: 'in_person', is_headquarters: false, address_id: null, created_at: '', ...input }
        locations.push(row)
        return { ok: true, data: row }
      },
      async updateLocation(id: string, input: Record<string, unknown>) {
        calls.push(`updateLocation ${JSON.stringify(input)}`)
        return { ok: true, data: Object.assign(locations.find((l) => l.id === id)!, input) }
      },
    },
  } as unknown as LocationClient
  return { client, calls }
}

describe('org-locations', () => {
  test('create with city/state makes the address first and links it', async () => {
    const { client, calls } = fake()
    const r = await createLocationWithAddress(client, 'o1', { name: 'Main Campus' }, { city: ' Arlington ', state: 'VA' })
    expect(r.ok).toBe(true)
    expect(calls[0]).toStartWith('addresses.create')
    expect(calls[0]).toContain('"name":"Main Campus"')
    expect(calls[0]).toContain('"city":"Arlington"')
    expect(calls[1]).toContain('"address_id":"a1"')
    if (r.ok) expect(locationLabel(r.data)).toBe('Main Campus (Arlington, VA)')
  })

  test('create with only blank fields makes no address', async () => {
    const { client, calls } = fake()
    const r = await createLocationWithAddress(client, 'o1', { name: 'Online', modality: 'remote' }, { city: '', state: '  ' })
    expect(calls).toEqual(['createLocation {"name":"Online","modality":"remote","address_id":null}'])
    if (r.ok) expect(locationLabel(r.data)).toBe('Online')
  })

  test('country_code is upper-cased', async () => {
    const { client, calls } = fake()
    await createLocationWithAddress(client, 'o1', { name: 'X' }, { country_code: ' gb ' })
    expect(calls[0]).toContain('"country_code":"GB"')
  })

  test('update edits the existing address in place', async () => {
    const { client, calls } = fake()
    const created = await createLocationWithAddress(client, 'o1', { name: 'HQ' }, { city: 'Arlington', state: 'VA' })
    if (!created.ok) throw new Error('setup')
    const r = await updateLocationWithAddress(client, created.data, { name: 'HQ' }, { city: 'Alexandria', state: 'VA' })
    expect(r.ok && r.data.address?.city).toBe('Alexandria')
    expect(calls.filter((c) => c.startsWith('addresses.create'))).toHaveLength(1)
    expect(calls.filter((c) => c.startsWith('addresses.update a1'))).toHaveLength(1)
  })

  test('update on a location without an address creates one and links it', async () => {
    const { client, calls } = fake()
    const created = await createLocationWithAddress(client, 'o1', { name: 'Remote' }, null)
    if (!created.ok) throw new Error('setup')
    const r = await updateLocationWithAddress(client, created.data, {}, { city: 'Austin', state: 'TX' })
    expect(r.ok && r.data.address?.city).toBe('Austin')
    expect(calls[1]).toContain('addresses.create')
    expect(calls[1]).toContain('"name":"Remote"')
    expect(calls[2]).toContain('"address_id":"a2"')
  })

  test('update with every field cleared sends address_id: null', async () => {
    const { client, calls } = fake()
    const created = await createLocationWithAddress(client, 'o1', { name: 'HQ' }, { city: 'Arlington' })
    if (!created.ok) throw new Error('setup')
    const r = await updateLocationWithAddress(client, created.data, { name: 'HQ' }, { city: '', state: '' })
    expect(r.ok).toBe(true)
    expect(calls[calls.length - 1]).toBe('updateLocation {"name":"HQ","address_id":null}')
    if (r.ok) expect(r.data.address).toBeNull()
  })

  test('list attaches each address', async () => {
    const { client } = fake()
    await createLocationWithAddress(client, 'o1', { name: 'Main Campus' }, { city: 'London', state: 'ENG' })
    const r = await listLocationsWithAddress(client, 'o1')
    expect(r.ok && r.data.map(locationLabel)).toEqual(['Main Campus (London, ENG)'])
  })

  test('list survives an unreadable address with address: null', async () => {
    const seed = fake()
    await createLocationWithAddress(seed.client, 'o1', { name: 'Main Campus' }, { city: 'London' })
    const { client } = fake({ failGet: true })
    await createLocationWithAddress(client, 'o1', { name: 'Main Campus' }, { city: 'London' })
    const r = await listLocationsWithAddress(client, 'o1')
    expect(r.ok).toBe(true)
    if (r.ok) {
      expect(r.data[0].address).toBeNull()
      expect(locationLabel(r.data[0])).toBe('Main Campus')
    }
  })

  test('a failed listLocations comes back as { ok: false }', async () => {
    const { client } = fake({ failList: true })
    const r = await listLocationsWithAddress(client, 'o1')
    expect(r.ok).toBe(false)
  })

  test('locationLabel without an address is the name', () => {
    const l = { name: 'Main Campus', address: null } as unknown as LocationWithAddress
    expect(locationLabel(l)).toBe('Main Campus')
  })
})
