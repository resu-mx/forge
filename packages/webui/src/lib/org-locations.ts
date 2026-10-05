/**
 * Org locations (formerly campuses) together with their address.
 *
 * Decision (resu-mx/forge#31, shared with #32): the location forms' flat street/city/state/zip/
 * country inputs are stored as an `addresses` row linked by `org_locations.address_id`, the way
 * migration 047 moved the old campus columns. Labels read the address, never `location.city`.
 *
 * No `$lib` imports: the client is a parameter, so `bun test` can run this with a fake.
 */
import type {
  Address,
  CreateOrgLocation,
  ForgeClient,
  OrgLocation,
  Result,
  UpdateOrgLocation,
} from '@forge/sdk'

export interface LocationAddressFields {
  street_1?: string | null
  city?: string | null
  state?: string | null
  zip?: string | null
  country_code?: string | null
}

export type LocationWithAddress = OrgLocation & { address: Address | null }
export type LocationClient = Pick<ForgeClient, 'organizations' | 'addresses'>

const KEYS = ['street_1', 'city', 'state', 'zip', 'country_code'] as const
type Normalised = Record<(typeof KEYS)[number], string | null>

/** Trimmed values; '' and missing become null; country_code is upper-cased. */
function normalise(fields: LocationAddressFields | null | undefined): Normalised {
  const out = {} as Normalised
  for (const k of KEYS) {
    const v = fields?.[k]?.trim() ?? ''
    out[k] = v === '' ? null : k === 'country_code' ? v.toUpperCase() : v
  }
  return out
}

export function hasAddress(fields: LocationAddressFields | null | undefined): boolean {
  return Object.values(normalise(fields)).some((v) => v !== null)
}

export function formatAddress(a: Pick<Address, 'city' | 'state'> | null | undefined): string {
  if (!a) return ''
  return [a.city, a.state].filter(Boolean).join(', ')
}

export function locationLabel(l: LocationWithAddress): string {
  const where = formatAddress(l.address)
  return where ? `${l.name} (${where})` : l.name
}

export async function listLocationsWithAddress(
  forge: LocationClient,
  orgId: string,
): Promise<Result<LocationWithAddress[]>> {
  const res = await forge.organizations.listLocations(orgId)
  if (!res.ok) return res
  const ids = [...new Set(res.data.map((l) => l.address_id).filter((id): id is string => !!id))]
  const got = await Promise.all(ids.map((id) => forge.addresses.get(id)))
  const byId = new Map<string, Address>()
  got.forEach((r, i) => {
    if (r.ok) byId.set(ids[i], r.data) // an unreadable address only costs the label
  })
  return {
    ok: true,
    data: res.data.map((l) => ({ ...l, address: (l.address_id && byId.get(l.address_id)) || null })),
  }
}

export async function createLocationWithAddress(
  forge: LocationClient,
  orgId: string,
  input: CreateOrgLocation,
  fields: LocationAddressFields | null,
): Promise<Result<LocationWithAddress>> {
  let address: Address | null = null
  if (hasAddress(fields)) {
    const n = normalise(fields)
    const a = await forge.addresses.create({ name: input.name, ...n, country_code: n.country_code ?? undefined })
    if (!a.ok) return a
    address = a.data
  }
  const loc = await forge.organizations.createLocation(orgId, { ...input, address_id: address?.id ?? null })
  if (!loc.ok) {
    // TODO: best-effort `forge.addresses.delete(address.id)` so a failed create leaves no orphan row.
    return loc
  }
  return { ok: true, data: { ...loc.data, address } }
}

export async function updateLocationWithAddress(
  forge: LocationClient,
  current: LocationWithAddress,
  patch: UpdateOrgLocation,
  fields: LocationAddressFields | null,
): Promise<Result<LocationWithAddress>> {
  const n = normalise(fields)
  let address = current.address
  let addressId: string | null | undefined // undefined = leave address_id alone
  if (hasAddress(fields)) {
    if (current.address_id) {
      // Cleared fields go as null. TODO: UpdateAddress.country_code is not nullable; decide whether
      // clearing the country should reset it to 'US'.
      const a = await forge.addresses.update(current.address_id, { ...n, country_code: n.country_code ?? undefined })
      if (!a.ok) return a
      address = a.data
    } else {
      const a = await forge.addresses.create({
        name: patch.name ?? current.name,
        ...n,
        country_code: n.country_code ?? undefined,
      })
      if (!a.ok) return a
      address = a.data
      addressId = a.data.id
    }
  } else if (current.address_id) {
    addressId = null // unlink. TODO: delete the now-unused address row (best effort).
    address = null
  }
  const loc = await forge.organizations.updateLocation(
    current.id,
    addressId === undefined ? patch : { ...patch, address_id: addressId },
  )
  if (!loc.ok) return loc
  return { ok: true, data: { ...loc.data, address } }
}

/** "City, ST" of the org's headquarters location; '' when it has none, or no address. */
export async function hqLocationLabel(forge: LocationClient, orgId: string): Promise<Result<string>> {
  const res = await forge.organizations.listLocations(orgId)
  if (!res.ok) return res
  // Rust sends is_headquarters as 0/1, TS as a boolean.
  const hq = res.data.find((l) => !!l.is_headquarters)
  if (!hq?.address_id) return { ok: true, data: '' }
  const addr = await forge.addresses.get(hq.address_id)
  return addr.ok ? { ok: true, data: formatAddress(addr.data) } : addr
}
