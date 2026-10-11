/**
 * Live R2 round trip against the PREVIEW bucket. Opt-in: runs only with FORGE_R2_ROUNDTRIP=1
 * and R2 credentials in the environment (`just demo-data roundtrip` provides both).
 *
 * It refuses any bucket whose name does not end in `-preview`, writes only under a fresh
 * random uuid, tags the object with `forge-test-run`, and always deletes what it wrote
 * (the preview bucket's lifecycle rule is the backstop).
 */

import { Database } from 'bun:sqlite'
import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { AwsClient } from 'aws4fetch'
import { writeFileSync } from 'fs'
import { join } from 'path'
import { tempDir } from '../../__tests__/helpers'
import { readDatasetMeta } from '../../postpass/meta'
import { inspectDatasetBytes } from '../commands'
import { type R2Config, assertPreviewBucket, hasCredentials, loadR2Config, missingCredentials } from '../config'
import { PreconditionFailedError, R2Error, R2Objects, SQLITE_CONTENT_TYPE, sha256Hex } from '../objects'
import { FixtureFactory } from './fixture'

const enabled = Boolean(process.env.FORGE_R2_ROUNDTRIP)
const creds = hasCredentials()

// Asked to run but cannot: fail loudly instead of skipping to a vacuous green.
test.if(enabled && !creds)('FORGE_R2_ROUNDTRIP is set, so R2 credentials must be too', () => {
  throw new Error(`missing ${missingCredentials().join(', ')}`)
})

describe.skipIf(!enabled || !creds)('R2 round trip (preview bucket only)', () => {
  // The describe body runs even when skipped, so everything that needs credentials is
  // created in beforeAll.
  let config: R2Config
  let objects: R2Objects
  let work: { dir: string; cleanup(): void } | undefined
  const runId = crypto.randomUUID()
  const uuid = crypto.randomUUID()
  const key = `user/${uuid}/data.sqlite`
  // A second fresh key for the must-fail control (a body that does not match its hash).
  const controlKey = `user/${crypto.randomUUID()}/data.sqlite`
  let bytes: Uint8Array
  let meta: Record<string, string>

  // Checked before every network call, so no step can touch a non-preview bucket.
  const preview = () => assertPreviewBucket(config.bucket)

  beforeAll(() => {
    config = loadR2Config('preview')
    preview()
    objects = new R2Objects(config)
    work = tempDir('r2-roundtrip')
    const fx = new FixtureFactory(work.dir).variant({ uuid, persona: 'roundtrip-test' })
    bytes = fx.bytes
    meta = {
      'forge-kind': 'generated',
      'forge-dataset-uuid': uuid,
      'forge-sha256': fx.manifest.sha256,
      'forge-schema-head': fx.manifest.schema_head,
      'forge-test-run': runId,
    }
  })

  afterAll(async () => {
    try {
      if (objects) {
        preview()
        await objects.delete(key)
        await objects.delete(controlKey)
      }
    } finally {
      work?.cleanup()
    }
  })

  test('the bucket is a preview bucket', () => {
    expect(config.bucket.endsWith('-preview')).toBe(true)
  })

  test('put with If-None-Match: * creates the object', async () => {
    preview()
    expect(await objects.head(key)).toBeNull()
    await objects.put(key, bytes, meta, { ifNoneMatch: true })
  })

  test('a second create-only put is refused (R2 honours If-None-Match: *)', async () => {
    preview()
    await expect(objects.put(key, bytes, meta, { ifNoneMatch: true })).rejects.toBeInstanceOf(PreconditionFailedError)
  })

  test('a body that does not match X-Amz-Content-Sha256 is rejected (control)', async () => {
    preview()
    const aws = new AwsClient({
      accessKeyId: config.credentials.accessKeyId,
      secretAccessKey: config.credentials.secretAccessKey,
      service: 's3',
      region: 'auto',
      retries: 0,
    })
    const req = await aws.sign(objects.url(controlKey), {
      method: 'PUT',
      headers: { 'content-type': SQLITE_CONTENT_TYPE, 'x-amz-content-sha256': sha256Hex(new TextEncoder().encode('not these bytes')) },
      body: bytes as Uint8Array<ArrayBuffer>,
    })
    const res = await fetch(req)
    const body = await res.text()
    expect(res.status).toBe(400)
    expect(body).toMatch(/<Code>[A-Za-z0-9]+<\/Code>/)
    expect(await objects.head(controlKey)).toBeNull()
  })

  test('head returns the metadata and size that were put', async () => {
    preview()
    const head = await objects.head(key)
    expect(head).not.toBeNull()
    expect(head?.size).toBe(bytes.byteLength)
    expect(head?.contentType).toBe(SQLITE_CONTENT_TYPE)
    for (const [k, v] of Object.entries(meta)) expect(head?.meta[k]).toBe(v)
  })

  test('list finds the key', async () => {
    preview()
    const entries = await objects.list(`user/${uuid}/`)
    expect(entries.map((e) => e.key)).toEqual([key])
    expect(entries[0]?.size).toBe(bytes.byteLength)
  })

  test('get returns the same bytes, and the file opens as a generated dataset', async () => {
    preview()
    const got = await objects.get(key)
    expect(got).not.toBeNull()
    const downloaded = got?.bytes as Uint8Array
    expect(sha256Hex(downloaded)).toBe(meta['forge-sha256'] as string)
    expect(got?.head.meta['forge-test-run']).toBe(runId)

    const path = join((work as { dir: string }).dir, 'downloaded.sqlite')
    writeFileSync(path, downloaded)
    const db = new Database(path, { readonly: true })
    try {
      expect(readDatasetMeta(db).kind).toBe('generated')
      expect(db.query('PRAGMA integrity_check').all()).toEqual([{ integrity_check: 'ok' }])
    } finally {
      db.close()
    }
    expect(inspectDatasetBytes(downloaded, uuid)).toEqual([])
  })

  test('delete removes it: head returns null', async () => {
    preview()
    await objects.delete(key)
    expect(await objects.head(key)).toBeNull()
    expect(await objects.get(key)).toBeNull()
  })

  test('errors carry the status and S3 code, never a credential', async () => {
    preview()
    const bad = new R2Objects(loadR2Config('preview', { ...process.env, R2_SECRET_ACCESS_KEY: 'wrong-secret-for-the-test' }), { retries: 0 })
    const err = await bad.get(key).catch((e: unknown) => e)
    expect(err).toBeInstanceOf(R2Error)
    const message = (err as Error).message
    expect(message).toMatch(/HTTP 403/)
    expect(message).not.toContain('wrong-secret-for-the-test')
    expect(message).not.toContain(config.credentials.secretAccessKey)
    expect(message).not.toContain(config.credentials.accessKeyId)
  })
})
