/**
 * R2 object metadata (`x-amz-meta-*`) for a published dataset. It repeats the `dataset_meta`
 * marker plus the file's hashes, so `head` answers "what is this object?" without a download.
 *
 * S3 metadata travels as HTTP headers, so every key and value must be plain printable ASCII
 * with no leading or trailing space (servers trim header values, which would break the
 * signature). R2 caps custom metadata at 8 KiB; ours is a few hundred bytes.
 */

import type { Manifest } from '../postpass/manifest'

export const META_PREFIX = 'x-amz-meta-'

export const R2_META_KEYS = [
  'forge-kind',
  'forge-generator',
  'forge-generator-version',
  'forge-seed',
  'forge-persona',
  'forge-dataset-uuid',
  'forge-as-of',
  'forge-generated-at',
  'forge-schema-head',
  'forge-sha256',
  'forge-content-fingerprint',
] as const

export type R2MetaKey = (typeof R2_META_KEYS)[number]
export type R2Meta = Record<R2MetaKey, string>

/** Total encoded size limit (header names plus values) for custom metadata. */
export const MAX_META_BYTES = 8 * 1024

const META_KEY = /^[a-z0-9][a-z0-9-]{0,63}$/
const META_VALUE = /^[\x21-\x7e](?:[\x20-\x7e]{0,1022}[\x21-\x7e])?$/

/** Which `dataset_meta` key each marker field of the object metadata repeats. */
const FROM_DATASET_META: ReadonlyArray<[R2MetaKey, string]> = [
  ['forge-kind', 'kind'],
  ['forge-generator', 'generator'],
  ['forge-generator-version', 'generator_version'],
  ['forge-seed', 'seed'],
  ['forge-persona', 'persona'],
  ['forge-dataset-uuid', 'dataset_uuid'],
  ['forge-as-of', 'as_of'],
  ['forge-generated-at', 'generated_at'],
  ['forge-schema-head', 'schema_head'],
]

/** The manifest field that must agree with each `dataset_meta` key. */
const MANIFEST_FIELD: Readonly<Record<string, keyof Manifest>> = {
  generator: 'generator',
  generator_version: 'generator_version',
  seed: 'seed',
  persona: 'persona',
  dataset_uuid: 'uuid',
  as_of: 'as_of',
  generated_at: 'generated_at',
  schema_head: 'schema_head',
}

/**
 * Object metadata for a dataset, from its `dataset_meta` rows (the marker) and its manifest
 * (the hashes). Throws if the two disagree on a field they share.
 */
export function buildR2Meta(manifest: Manifest, datasetMeta: Record<string, string>): R2Meta {
  const meta = {} as R2Meta
  for (const [metaKey, dbKey] of FROM_DATASET_META) {
    const value = datasetMeta[dbKey]
    if (value === undefined || value === '') throw new Error(`dataset_meta.${dbKey} is missing`)
    const field = MANIFEST_FIELD[dbKey]
    if (field !== undefined && String(manifest[field]) !== value) {
      throw new Error(`dataset_meta.${dbKey} (${value}) disagrees with manifest.${field} (${String(manifest[field])})`)
    }
    meta[metaKey] = value
  }
  meta['forge-sha256'] = manifest.sha256
  meta['forge-content-fingerprint'] = manifest.content_fingerprint
  validateMeta(meta)
  return meta
}

/** Encoded size: every `x-amz-meta-<key>` header name plus its value. */
export function encodedMetaSize(meta: Record<string, string>): number {
  return Object.entries(meta).reduce((n, [k, v]) => n + META_PREFIX.length + k.length + v.length, 0)
}

/** Throws unless every key and value is header-safe ASCII and the total fits R2's limit. */
export function validateMeta(meta: Record<string, string>): void {
  for (const [key, value] of Object.entries(meta)) {
    if (!META_KEY.test(key)) throw new Error(`metadata key ${JSON.stringify(key)} must be lowercase ASCII letters, digits and hyphens`)
    if (typeof value !== 'string' || !META_VALUE.test(value)) {
      throw new Error(`metadata ${key} must be 1-1024 printable ASCII characters with no leading or trailing space`)
    }
  }
  const size = encodedMetaSize(meta)
  if (size >= MAX_META_BYTES) throw new Error(`metadata is ${size} bytes encoded; R2 allows under ${MAX_META_BYTES}`)
}

/** `{ 'forge-kind': 'generated' }` → `{ 'x-amz-meta-forge-kind': 'generated' }`, validated. */
export function metaHeaders(meta: Record<string, string>): Record<string, string> {
  validateMeta(meta)
  return Object.fromEntries(Object.entries(meta).map(([k, v]) => [`${META_PREFIX}${k}`, v]))
}

/** The `x-amz-meta-*` headers of a response, keyed without the prefix. */
export function parseMetaHeaders(headers: Headers): Record<string, string> {
  const meta: Record<string, string> = {}
  headers.forEach((value, name) => {
    const lower = name.toLowerCase()
    if (lower.startsWith(META_PREFIX)) meta[lower.slice(META_PREFIX.length)] = value
  })
  return Object.fromEntries(Object.entries(meta).sort(([a], [b]) => a.localeCompare(b)))
}
