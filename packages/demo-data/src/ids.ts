/**
 * Deterministic dataset ids.
 *
 * Every persona's dataset uuid is a UUIDv5 under a fixed demo namespace, so it is stable
 * across seeds, `--as-of` values and generator versions: re-generating a persona always
 * targets the same `user/<uuid>/data.sqlite` key.
 */

/** Name of the demo namespace, hashed under the RFC 4122 DNS namespace. */
export const DEMO_NS_NAME = 'demo-data.resu.mx'

/** `uuidv5('demo-data.resu.mx', DNS)`. */
export const DEMO_NS: string = Bun.randomUUIDv5(DEMO_NS_NAME, 'dns')

/** The dataset uuid for a persona slug. */
export function personaUuid(slug: string): string {
  return Bun.randomUUIDv5(slug, DEMO_NS)
}

/** Matches a UUID anywhere in a string (any version, either case). */
export const UUID_PATTERN = /[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/g

/** A canonical lowercase UUID and nothing else. */
export function isCanonicalUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(value)
}
