/**
 * Offline R2 tests: configuration, key validation, request signing, metadata, and the push
 * guards and push/pull flow against an in-memory store. No network.
 */

import { afterAll, beforeAll, describe, expect, setSystemTime, test } from 'bun:test'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs'
import { join } from 'path'
import { personaUuid } from '../../ids'
import { readDatasetMeta } from '../../postpass/meta'
import { tempDir } from '../../__tests__/helpers'
import {
  type ObjectStore,
  PullVerificationError,
  PushRefusedError,
  datasetFiles,
  importInstructions,
  pullDataset,
  pushDataset,
  resolveDatasetId,
} from '../commands'
import {
  type R2Config,
  DEFAULT_BUCKETS,
  R2ConfigError,
  assertPreviewBucket,
  bucketFor,
  describeConfig,
  loadR2Config,
  missingCredentials,
  parseTarget,
} from '../config'
import { checkPushGuards, registeredDemoUuids, repoSchemaHead } from '../guards'
import { MAX_META_BYTES, R2_META_KEYS, buildR2Meta, metaHeaders, parseMetaHeaders, validateMeta } from '../meta'
import { type ListEntry, type ObjectHead, type PutOptions, R2Objects, SQLITE_CONTENT_TYPE, sha256Hex, userKey, uuidFromUserKey } from '../objects'
import { FixtureFactory } from './fixture'

const PERSONA = 'early-career-developer'
const PERSONA_UUID = personaUuid(PERSONA)
const SECRET = 'test-secret-key-that-must-never-be-printed-0123456789abcdef'
const ENV = {
  R2_ACCOUNT_ID: '0123456789abcdef0123456789abcdef',
  R2_ACCESS_KEY_ID: 'AKIDFORGETEST',
  R2_SECRET_ACCESS_KEY: SECRET,
}

function config(target: 'preview' | 'prod' = 'preview'): R2Config {
  return loadR2Config(target, ENV)
}

/** Parses `AWS4-HMAC-SHA256 Credential=…, SignedHeaders=a;b, Signature=…`. */
function authParts(req: Request): { credential: string; signedHeaders: string[]; signature: string } {
  const auth = req.headers.get('authorization') ?? ''
  const m = /^AWS4-HMAC-SHA256 Credential=([^,]+), SignedHeaders=([^,]+), Signature=([0-9a-f]{64})$/.exec(auth)
  if (!m) throw new Error(`unexpected authorization header shape: ${auth.slice(0, 40)}`)
  return { credential: m[1] as string, signedHeaders: (m[2] as string).split(';'), signature: m[3] as string }
}

describe('config', () => {
  test('missing credentials are named, never valued', () => {
    expect(missingCredentials({})).toEqual(['R2_ACCOUNT_ID', 'R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY'])
    expect(() => loadR2Config('preview', { R2_ACCOUNT_ID: ENV.R2_ACCOUNT_ID })).toThrow(/missing R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY/)
  })

  test('a malformed account id is rejected without echoing it', () => {
    const bad = 'not-an-account-id-but-maybe-a-secret'
    let message = ''
    try {
      loadR2Config('preview', { ...ENV, R2_ACCOUNT_ID: bad })
    } catch (e) {
      message = (e as Error).message
    }
    expect(message).toContain('R2_ACCOUNT_ID')
    expect(message).not.toContain(bad)
  })

  test('the secret key is not enumerable, serialisable or inspectable', () => {
    const cfg = config()
    expect(cfg.credentials.secretAccessKey).toBe(SECRET)
    expect(JSON.stringify(cfg)).not.toContain(SECRET)
    expect(Bun.inspect(cfg)).not.toContain(SECRET)
    expect(Bun.inspect(cfg)).not.toContain(ENV.R2_ACCESS_KEY_ID)
    expect(String(cfg.credentials)).not.toContain(SECRET)
    expect(`${cfg.credentials}`).not.toContain(ENV.R2_ACCESS_KEY_ID)
    expect(String(Object.values({ ...cfg.credentials }))).not.toContain(SECRET)
    expect(describeConfig(cfg)).toBe('preview bucket resumx-user-data-preview')
  })

  test('buckets default per target and can be overridden', () => {
    expect(bucketFor('prod', {})).toBe(DEFAULT_BUCKETS.prod)
    expect(bucketFor('preview', {})).toBe('resumx-user-data-preview')
    expect(bucketFor('preview', { R2_BUCKET_PREVIEW: 'other-preview' })).toBe('other-preview')
    expect(bucketFor('prod', { R2_BUCKET_PROD: 'other-prod' })).toBe('other-prod')
    expect(() => bucketFor('preview', { R2_BUCKET_PREVIEW: 'Bad_Bucket' })).toThrow(R2ConfigError)
  })

  test('targets are preview or prod', () => {
    expect(parseTarget('preview')).toBe('preview')
    expect(parseTarget('prod')).toBe('prod')
    expect(() => parseTarget('production')).toThrow(/--target/)
    expect(() => parseTarget(undefined)).toThrow(/--target/)
  })

  test('the round-trip guard only accepts -preview buckets', () => {
    expect(() => assertPreviewBucket('resumx-user-data-preview')).not.toThrow()
    expect(() => assertPreviewBucket('resumx-user-data')).toThrow(/-preview/)
    expect(() => assertPreviewBucket('resumx-user-data-preview-not')).toThrow()
  })
})

describe('userKey', () => {
  test('builds user/<uuid>/data.sqlite for a canonical lowercase uuid', () => {
    expect(userKey(PERSONA_UUID)).toBe(`user/${PERSONA_UUID}/data.sqlite`)
    expect(uuidFromUserKey(userKey(PERSONA_UUID))).toBe(PERSONA_UUID)
  })

  test.each([
    ['uppercase', PERSONA_UUID.toUpperCase()],
    ['braced', `{${PERSONA_UUID}}`],
    ['no hyphens', PERSONA_UUID.replaceAll('-', '')],
    ['trailing text', `${PERSONA_UUID}/../x`],
    ['path traversal', '../../etc/passwd'],
    ['leading space', ` ${PERSONA_UUID}`],
    ['empty', ''],
    ['urn', `urn:uuid:${PERSONA_UUID}`],
  ])('rejects %s', (_label, value) => {
    expect(() => userKey(value)).toThrow(/canonical lowercase UUID/)
  })

  test('rejects non-strings', () => {
    expect(() => userKey(undefined as unknown as string)).toThrow()
    expect(() => userKey(42 as unknown as string)).toThrow()
  })

  test('uuidFromUserKey only accepts the dataset key shape', () => {
    expect(uuidFromUserKey(`user/${PERSONA_UUID}/other.sqlite`)).toBeNull()
    expect(uuidFromUserKey(`user/${PERSONA_UUID.toUpperCase()}/data.sqlite`)).toBeNull()
    expect(uuidFromUserKey(`x/user/${PERSONA_UUID}/data.sqlite`)).toBeNull()
  })
})

describe('metadata', () => {
  const meta = Object.fromEntries(R2_META_KEYS.map((k) => [k, `value-of-${k}`]))

  test('headers carry the x-amz-meta- prefix and parse back', () => {
    const headers = metaHeaders(meta)
    expect(Object.keys(headers).every((h) => h.startsWith('x-amz-meta-forge-'))).toBe(true)
    expect(parseMetaHeaders(new Headers({ ...headers, 'content-type': 'x' }))).toEqual(
      Object.fromEntries(Object.entries(meta).sort(([a], [b]) => a.localeCompare(b))),
    )
  })

  test.each([
    ['non-ASCII value', { 'forge-persona': 'café' }],
    ['leading space', { 'forge-persona': ' x' }],
    ['trailing space', { 'forge-persona': 'x ' }],
    ['control character', { 'forge-persona': 'a\nb' }],
    ['empty value', { 'forge-persona': '' }],
    ['uppercase key', { 'Forge-Persona': 'x' }],
    ['key with underscore', { forge_persona: 'x' }],
  ])('rejects %s', (_label, bad) => {
    expect(() => validateMeta(bad as Record<string, string>)).toThrow()
  })

  test('rejects metadata at or over 8 KiB encoded', () => {
    const big: Record<string, string> = {}
    for (let i = 0; i < 9; i++) big[`k${i}`] = 'x'.repeat(1000)
    expect(() => validateMeta(big)).toThrow(new RegExp(String(MAX_META_BYTES)))
  })
})

describe('signing (aws4fetch, no network)', () => {
  const bytes = new TextEncoder().encode('SQLite format 3\u0000 pretend payload')
  const meta = Object.fromEntries(R2_META_KEYS.map((k) => [k, `v-${k}`]))
  const key = userKey(PERSONA_UUID)

  beforeAll(() => setSystemTime(new Date('2026-10-01T12:00:00Z')))
  afterAll(() => setSystemTime())

  test('PUT goes to the path-style R2 endpoint with the SQLite content type', async () => {
    const req = await new R2Objects(config()).signPut(key, bytes, meta)
    expect(req.method).toBe('PUT')
    expect(req.url).toBe(`https://${ENV.R2_ACCOUNT_ID}.r2.cloudflarestorage.com/resumx-user-data-preview/${key}`)
    expect(req.headers.get('content-type')).toBe(SQLITE_CONTENT_TYPE)
    expect(new Uint8Array(await req.arrayBuffer())).toEqual(bytes)
  })

  test('every metadata header, the content hash and the date are in SignedHeaders', async () => {
    const req = await new R2Objects(config()).signPut(key, bytes, meta)
    const { credential, signedHeaders } = authParts(req)
    for (const k of R2_META_KEYS) expect(signedHeaders).toContain(`x-amz-meta-${k}`)
    expect(signedHeaders).toContain('x-amz-content-sha256')
    expect(signedHeaders).toContain('x-amz-date')
    expect(signedHeaders).toContain('host')
    expect(signedHeaders).not.toContain('if-none-match')
    expect(credential).toBe(`${ENV.R2_ACCESS_KEY_ID}/20261001/auto/s3/aws4_request`)
    for (const k of R2_META_KEYS) expect(req.headers.get(`x-amz-meta-${k}`)).toBe(meta[k] as string)
  })

  test('the payload hash is the real sha256, so R2 can verify the body', async () => {
    const req = await new R2Objects(config()).signPut(key, bytes, meta)
    expect(req.headers.get('x-amz-content-sha256')).toBe(sha256Hex(bytes))
    expect(req.headers.get('x-amz-content-sha256')).not.toBe('UNSIGNED-PAYLOAD')
  })

  test('If-None-Match: * is sent and signed when asked for', async () => {
    const req = await new R2Objects(config()).signPut(key, bytes, meta, { ifNoneMatch: true })
    expect(req.headers.get('if-none-match')).toBe('*')
    expect(authParts(req).signedHeaders).toContain('if-none-match')
  })

  test('the signature covers metadata values and the body', async () => {
    const r2 = new R2Objects(config())
    const a = authParts(await r2.signPut(key, bytes, meta)).signature
    const again = authParts(await r2.signPut(key, bytes, meta)).signature
    const otherMeta = authParts(await r2.signPut(key, bytes, { ...meta, 'forge-seed': 'different' })).signature
    const otherBody = authParts(await r2.signPut(key, new TextEncoder().encode('other bytes'), meta)).signature
    expect(again).toBe(a)
    expect(otherMeta).not.toBe(a)
    expect(otherBody).not.toBe(a)
  })

  test('the secret key never appears in a signed request', async () => {
    const req = await new R2Objects(config()).signPut(key, bytes, meta, { ifNoneMatch: true })
    const parts: string[] = [req.url]
    req.headers.forEach((value, name) => parts.push(name, value))
    const all = parts.join('\n')
    expect(all).not.toContain(SECRET)
  })

  test('HEAD, GET and DELETE are signed for the same object', async () => {
    const r2 = new R2Objects(config())
    for (const method of ['HEAD', 'GET', 'DELETE'] as const) {
      const req = await r2.signRequest(method, key)
      expect(req.method).toBe(method)
      expect(req.url).toEndWith(`/resumx-user-data-preview/${key}`)
      expect(authParts(req).signedHeaders).toContain('x-amz-date')
    }
  })

  test('put maps 412 to PreconditionFailedError and head maps 404 to null', async () => {
    const r2 = new R2Objects(config(), {
      retries: 0,
      fetch: async (req) =>
        req.method === 'PUT'
          ? new Response('<Error><Code>PreconditionFailed</Code><Message>At least one of the pre-conditions you specified did not hold</Message></Error>', { status: 412 })
          : new Response(null, { status: 404 }),
    })
    await expect(r2.put(key, bytes, meta, { ifNoneMatch: true })).rejects.toThrow(/PreconditionFailed/)
    expect(await r2.head(key)).toBeNull()
  })

  test('network failures are reported without the account id or keys', async () => {
    const r2 = new R2Objects(config(), {
      retries: 1,
      fetch: async (req) => {
        throw new Error(`Unable to connect: ${req.url} (key ${ENV.R2_ACCESS_KEY_ID}, ${SECRET})`)
      },
    })
    const message = await r2.head(key).then(
      () => '',
      (e: Error) => e.message,
    )
    expect(message).toMatch(/failed after 2 attempt\(s\)/)
    expect(message).toContain('<R2_ACCOUNT_ID>')
    for (const value of Object.values(ENV)) expect(message).not.toContain(value)
  })

  test('HTTP errors carry the status and S3 code, redacted', async () => {
    const r2 = new R2Objects(config(), {
      retries: 0,
      fetch: async () => new Response(`<Error><Code>AccessDenied</Code><Message>key ${ENV.R2_ACCESS_KEY_ID} denied</Message></Error>`, { status: 403 }),
    })
    const message = await r2.get(key).then(
      () => '',
      (e: Error) => e.message,
    )
    expect(message).toMatch(/HTTP 403 AccessDenied/)
    expect(message).not.toContain(ENV.R2_ACCESS_KEY_ID)
  })

  test('head returns size and x-amz-meta-* values', async () => {
    const r2 = new R2Objects(config(), {
      fetch: async () =>
        new Response(null, {
          status: 200,
          headers: { 'content-length': '1234', etag: '"abc"', 'content-type': SQLITE_CONTENT_TYPE, 'x-amz-meta-forge-kind': 'generated' },
        }),
    })
    const head = await r2.head(key)
    expect(head?.size).toBe(1234)
    expect(head?.meta).toEqual({ 'forge-kind': 'generated' })
    expect(head?.contentType).toBe(SQLITE_CONTENT_TYPE)
  })
})

// ─── Push guards ──────────────────────────────────────────

let factory: FixtureFactory
let work: { dir: string; cleanup(): void }
beforeAll(() => {
  work = tempDir('r2')
  factory = new FixtureFactory(work.dir)
})
afterAll(() => work.cleanup())

function guard(fx: { bytes: Uint8Array; manifest: import('../../postpass/manifest').Manifest }, key = userKey(fx.manifest.uuid), extra: { allowStale?: boolean; head?: string } = {}) {
  return checkPushGuards({ bytes: fx.bytes, manifest: fx.manifest, key, repoSchemaHead: extra.head ?? repoSchemaHead(), allowStale: extra.allowStale })
}

describe('push guards', () => {
  test('the repo schema head is the newest migration file', () => {
    expect(repoSchemaHead()).toMatch(/^\d{3}_[a-z0-9_]+$/)
  })

  test('registered uuids are exactly the personas', () => {
    expect(registeredDemoUuids().get(PERSONA_UUID)).toBe(PERSONA)
  })

  test('a generated, compacted, registered dataset passes (positive control)', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })
    const r = guard(fx)
    expect(r.problems).toEqual([])
    expect(r.datasetMeta.kind).toBe('generated')
    expect(r.sha256).toBe(fx.manifest.sha256)
  })

  test('rejects a plain migrated database with no marker', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA, meta: null })
    expect(guard(fx).problems.join('\n')).toMatch(/dataset_meta\.kind is missing/)
  })

  test('rejects a database marked as anything but generated', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA, meta: { kind: 'real' } })
    expect(guard(fx).problems.join('\n')).toMatch(/dataset_meta\.kind is "real"/)
  })

  test('rejects a WAL-mode file', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA, wal: true })
    expect(fx.bytes[18]).toBe(2)
    const problems = guard(fx).problems
    expect(problems).toHaveLength(1)
    expect(problems[0]).toMatch(/header bytes 18-19 are 2,2/)
  })

  test('rejects a dataset_uuid that does not match the key', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA, meta: { dataset_uuid: crypto.randomUUID() } })
    expect(guard(fx).problems.join('\n')).toMatch(/dataset_meta\.dataset_uuid .* does not match key uuid/)
  })

  test('rejects a manifest whose uuid does not match the key', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })
    const other = crypto.randomUUID()
    expect(guard(fx, userKey(other)).problems.join('\n')).toMatch(/manifest uuid .* does not match key uuid/)
  })

  test('rejects an unregistered uuid even when everything else agrees', () => {
    const stray = crypto.randomUUID()
    const fx = factory.variant({ uuid: stray, persona: PERSONA })
    const problems = guard(fx).problems
    expect(problems.join('\n')).toMatch(/is not a registered demo dataset uuid/)
  })

  test('rejects a persona that does not own the uuid', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: 'someone-else' })
    expect(guard(fx).problems.join('\n')).toMatch(/dataset_meta\.persona someone-else is not early-career-developer/)
  })

  test('rejects a tampered file (sha256 differs from the manifest)', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })
    const tampered = new Uint8Array(fx.bytes)
    // Flip one byte in the last page: still a readable SQLite file, but not the generated one.
    const i = tampered.byteLength - 10
    tampered[i] = (tampered[i] as number) ^ 0xff
    const problems = guard({ bytes: tampered, manifest: fx.manifest }).problems
    expect(problems.join('\n')).toMatch(/sha256 [0-9a-f]{64} does not match manifest/)
  })

  test('rejects a stale schema head unless --allow-stale', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })
    expect(guard(fx, undefined, { head: '999_future' }).problems.join('\n')).toMatch(/is not the repo's 999_future/)
    expect(guard(fx, undefined, { head: '999_future', allowStale: true }).problems).toEqual([])
  })

  test('rejects a schema head that disagrees with the applied migrations', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA, meta: { schema_head: '001_initial' } })
    expect(guard(fx, undefined, { allowStale: true }).problems.join('\n')).toMatch(/does not match the newest applied migration/)
  })

  test('rejects bytes that are not SQLite', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })
    const junk = new TextEncoder().encode('PK\u0003\u0004 definitely a zip file'.padEnd(200, '.'))
    expect(guard({ bytes: junk, manifest: fx.manifest }).problems).toEqual(['not a SQLite database (missing the "SQLite format 3" header)'])
  })

  test('object metadata is built from dataset_meta plus the manifest hashes', () => {
    const fx = factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })
    const meta = buildR2Meta(fx.manifest, guard(fx).datasetMeta)
    expect(Object.keys(meta).sort()).toEqual([...R2_META_KEYS].sort())
    expect(meta['forge-kind']).toBe('generated')
    expect(meta['forge-dataset-uuid']).toBe(PERSONA_UUID)
    expect(meta['forge-sha256']).toBe(fx.manifest.sha256)
    expect(() => buildR2Meta({ ...fx.manifest, seed: 'other' }, guard(fx).datasetMeta)).toThrow(/disagrees with manifest\.seed/)
  })
})

// ─── Push and pull against an in-memory store ─────────────

class MemoryStore implements ObjectStore {
  readonly bucket = 'memory-preview'
  readonly objects = new Map<string, { bytes: Uint8Array; meta: Record<string, string> }>()
  readonly puts: { key: string; opts: PutOptions }[] = []

  async put(key: string, bytes: Uint8Array, meta: Record<string, string>, opts: PutOptions = {}) {
    validateMeta(meta)
    this.puts.push({ key, opts })
    if (opts.ifNoneMatch && this.objects.has(key)) throw new Error('412')
    this.objects.set(key, { bytes: new Uint8Array(bytes), meta: { ...meta } })
    return { etag: '"x"' }
  }
  async head(key: string): Promise<ObjectHead | null> {
    const o = this.objects.get(key)
    return o ? { key, size: o.bytes.byteLength, etag: '"x"', lastModified: null, contentType: SQLITE_CONTENT_TYPE, meta: { ...o.meta } } : null
  }
  async get(key: string) {
    const head = await this.head(key)
    const o = this.objects.get(key)
    return head && o ? { bytes: new Uint8Array(o.bytes), head } : null
  }
  async delete(key: string) {
    this.objects.delete(key)
  }
  async list(prefix: string): Promise<ListEntry[]> {
    return [...this.objects.keys()].filter((k) => k.startsWith(prefix)).map((key) => ({ key, size: 0, lastModified: null, etag: null }))
  }
}

/** Write a fixture where `generate` would put it: `<out>/user/<uuid>/{data.sqlite,manifest.json}`. */
function stage(outDir: string, fx: { bytes: Uint8Array; manifest: import('../../postpass/manifest').Manifest }) {
  const files = datasetFiles(outDir, PERSONA)
  mkdirSync(join(outDir, 'user', PERSONA_UUID), { recursive: true })
  writeFileSync(files.dbPath, fx.bytes)
  writeFileSync(files.manifestPath, JSON.stringify(fx.manifest))
  return files
}

describe('push flow', () => {
  test('a refused dataset never reaches the store', async () => {
    const out = join(work.dir, 'out-refused')
    const files = stage(out, factory.variant({ uuid: PERSONA_UUID, persona: PERSONA, meta: null }))
    const store = new MemoryStore()
    await expect(pushDataset(store, files, {})).rejects.toBeInstanceOf(PushRefusedError)
    expect(store.puts).toEqual([])
  })

  test('a missing dataset is refused with the generate hint', async () => {
    const store = new MemoryStore()
    await expect(pushDataset(store, datasetFiles(join(work.dir, 'nothing-here'), PERSONA), {})).rejects.toThrow(/just demo-data generate early-career-developer/)
    expect(store.puts).toEqual([])
  })

  test('uploads create-only, skips when up to date, re-uploads with --force', async () => {
    const out = join(work.dir, 'out-ok')
    const files = stage(out, factory.variant({ uuid: PERSONA_UUID, persona: PERSONA }))
    const store = new MemoryStore()

    const first = await pushDataset(store, files, {})
    expect(first.action).toBe('uploaded')
    expect(store.puts).toEqual([{ key: userKey(PERSONA_UUID), opts: { ifNoneMatch: true } }])
    const stored = store.objects.get(userKey(PERSONA_UUID))
    expect(stored?.meta['forge-kind']).toBe('generated')
    expect(sha256Hex(stored?.bytes as Uint8Array)).toBe(first.sha256)

    const second = await pushDataset(store, files, {})
    expect(second.action).toBe('skipped')
    expect(store.puts).toHaveLength(1)

    const forced = await pushDataset(store, files, { force: true })
    expect(forced.action).toBe('uploaded')
    expect(store.puts[1]).toEqual({ key: userKey(PERSONA_UUID), opts: { ifNoneMatch: false } })
  })

  test('refuses to overwrite an object that lacks the generated marker', async () => {
    const out = join(work.dir, 'out-foreign')
    const files = stage(out, factory.variant({ uuid: PERSONA_UUID, persona: PERSONA }))
    const store = new MemoryStore()
    store.objects.set(userKey(PERSONA_UUID), { bytes: new Uint8Array([1, 2, 3]), meta: {} })
    await expect(pushDataset(store, files, {})).rejects.toThrow(/without forge-kind=generated/)
    expect(store.puts).toEqual([])
  })
})

describe('pull flow', () => {
  async function seeded(): Promise<MemoryStore> {
    const store = new MemoryStore()
    const out = join(work.dir, `out-pull-${crypto.randomUUID()}`)
    await pushDataset(store, stage(out, factory.variant({ uuid: PERSONA_UUID, persona: PERSONA })), {})
    return store
  }

  test('ids resolve from a slug or a uuid', () => {
    expect(resolveDatasetId(PERSONA)).toEqual({ uuid: PERSONA_UUID, slug: PERSONA })
    expect(resolveDatasetId(PERSONA_UUID)).toEqual({ uuid: PERSONA_UUID, slug: PERSONA })
    expect(() => resolveDatasetId('no-such-persona')).toThrow(/neither a canonical lowercase uuid nor a persona/)
  })

  test('verifies and writes the file, then explains the import', async () => {
    const store = await seeded()
    const outPath = join(work.dir, 'pulled', 'a.sqlite')
    const r = await pullDataset(store, { id: PERSONA, outPath })
    expect(r.sha256).toBe(sha256Hex(new Uint8Array(readFileSync(outPath))))
    const { Database } = await import('bun:sqlite')
    const db = new Database(outPath, { readonly: true })
    expect(readDatasetMeta(db).kind).toBe('generated')
    db.close()
    expect(importInstructions(outPath)).toContain('Settings → Storage')
    await expect(pullDataset(store, { id: PERSONA, outPath })).rejects.toThrow(/exists; pass --force/)
  })

  test('a body that does not match forge-sha256 is rejected and nothing is written', async () => {
    const store = await seeded()
    const obj = store.objects.get(userKey(PERSONA_UUID))
    if (!obj) throw new Error('not seeded')
    obj.meta['forge-sha256'] = 'f'.repeat(64)
    const outPath = join(work.dir, 'pulled', 'tampered.sqlite')
    await expect(pullDataset(store, { id: PERSONA_UUID, outPath })).rejects.toBeInstanceOf(PullVerificationError)
    expect(existsSync(outPath)).toBe(false)
  })

  test('an object without the generated marker is rejected', async () => {
    const store = await seeded()
    const obj = store.objects.get(userKey(PERSONA_UUID))
    if (!obj) throw new Error('not seeded')
    delete obj.meta['forge-kind']
    await expect(pullDataset(store, { id: PERSONA, outPath: join(work.dir, 'pulled', 'unmarked.sqlite') })).rejects.toThrow(/forge-kind is \(missing\)/)
  })
})
