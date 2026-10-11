/**
 * Object operations against an R2 bucket over its S3 API.
 *
 * `put`, `head`, `get` and `delete` are SigV4-signed with `aws4fetch`, because
 * `Bun.S3Client` can neither set nor read `x-amz-meta-*` metadata. `list` uses
 * `Bun.S3Client`, which handles ListObjectsV2 and its XML.
 *
 * A put sends `X-Amz-Content-Sha256` with the body's real hash (not `UNSIGNED-PAYLOAD`), so
 * the signature covers the bytes and R2 rejects a body that does not match it.
 */

import { AwsClient } from 'aws4fetch'
import { isCanonicalUuid } from '../ids'
import { type R2Config, describeConfig, endpointOrigin, objectUrl, redact } from './config'
import { metaHeaders, parseMetaHeaders } from './meta'

export const SQLITE_CONTENT_TYPE = 'application/vnd.sqlite3'

/** `user/<uuid>/data.sqlite`; throws unless `uuid` is a canonical lowercase UUID. */
export function userKey(uuid: string): string {
  if (typeof uuid !== 'string' || !isCanonicalUuid(uuid)) {
    throw new Error(`not a canonical lowercase UUID: ${JSON.stringify(String(uuid).slice(0, 64))}`)
  }
  return `user/${uuid}/data.sqlite`
}

/** The uuid in a `user/<uuid>/data.sqlite` key, or null for any other key. */
export function uuidFromUserKey(key: string): string | null {
  const m = /^user\/([^/]+)\/data\.sqlite$/.exec(key)
  return m && isCanonicalUuid(m[1] as string) ? (m[1] as string) : null
}

export function sha256Hex(bytes: Uint8Array): string {
  return new Bun.CryptoHasher('sha256').update(bytes).digest('hex')
}

export interface ObjectHead {
  key: string
  size: number
  etag: string | null
  lastModified: string | null
  contentType: string | null
  /** `x-amz-meta-*` values, keyed without the prefix. */
  meta: Record<string, string>
}

export interface ListEntry {
  key: string
  size: number
  lastModified: string | null
  etag: string | null
}

export interface PutOptions {
  /** Send `If-None-Match: *`: create only, fail with `PreconditionFailedError` if the key exists. */
  ifNoneMatch?: boolean
}

export class R2Error extends Error {
  override name = 'R2Error'
  constructor(
    message: string,
    readonly status: number,
    readonly code: string | null,
  ) {
    super(message)
  }
}

/** The object already exists (a put with `If-None-Match: *` was refused with 412). */
export class PreconditionFailedError extends R2Error {
  override name = 'PreconditionFailedError'
}

export type FetchFn = (request: Request) => Promise<Response>

export interface R2ObjectsOptions {
  /** Replaces `fetch` (tests). */
  fetch?: FetchFn
  /** Retries after a network error, 429 or 5xx. Default 3. */
  retries?: number
}

/** Reads `<Code>` and `<Message>` from an S3 XML error body. */
async function errorDetail(res: Response): Promise<{ code: string | null; message: string | null }> {
  if (res.status === 304 || res.headers.get('content-length') === '0') return { code: null, message: null }
  const text = await res.text().catch(() => '')
  const code = /<Code>([^<]{1,128})<\/Code>/.exec(text)?.[1] ?? null
  const message = /<Message>([^<]{1,512})<\/Message>/.exec(text)?.[1] ?? null
  return { code, message }
}

function headFromResponse(key: string, res: Response): ObjectHead {
  return {
    key,
    size: Number(res.headers.get('content-length') ?? Number.NaN),
    etag: res.headers.get('etag'),
    lastModified: res.headers.get('last-modified'),
    contentType: res.headers.get('content-type'),
    meta: parseMetaHeaders(res.headers),
  }
}

export class R2Objects {
  private readonly aws: AwsClient
  private readonly doFetch: FetchFn
  private readonly retries: number

  constructor(
    readonly config: R2Config,
    opts: R2ObjectsOptions = {},
  ) {
    this.aws = new AwsClient({
      accessKeyId: config.credentials.accessKeyId,
      secretAccessKey: config.credentials.secretAccessKey,
      service: 's3',
      region: 'auto',
      retries: 0,
    })
    this.doFetch = opts.fetch ?? ((request) => fetch(request))
    this.retries = opts.retries ?? 3
  }

  get bucket(): string {
    return this.config.bucket
  }

  url(key: string): string {
    return objectUrl(this.config, key)
  }

  /** The signed PUT request, without sending it (exposed for the offline signing tests). */
  async signPut(key: string, bytes: Uint8Array, meta: Record<string, string>, opts: PutOptions = {}): Promise<Request> {
    const headers: Record<string, string> = {
      'content-type': SQLITE_CONTENT_TYPE,
      'x-amz-content-sha256': sha256Hex(bytes),
      ...metaHeaders(meta),
    }
    if (opts.ifNoneMatch) headers['if-none-match'] = '*'
    return this.aws.sign(this.url(key), { method: 'PUT', headers, body: bytes as Uint8Array<ArrayBuffer> })
  }

  async signRequest(method: 'GET' | 'HEAD' | 'DELETE', key: string): Promise<Request> {
    return this.aws.sign(this.url(key), { method })
  }

  /** Send a freshly signed request, retrying network errors, 429 and 5xx. */
  private async send(sign: () => Promise<Request>): Promise<{ res: Response; attempts: number }> {
    let lastError: unknown
    for (let attempt = 0; attempt <= this.retries; attempt++) {
      if (attempt > 0) await Bun.sleep(Math.min(4000, 200 * 2 ** (attempt - 1)) * (0.5 + Math.random() / 2))
      try {
        const res = await this.doFetch(await sign())
        if ((res.status === 429 || res.status >= 500) && attempt < this.retries) {
          await res.body?.cancel()
          lastError = new R2Error(`HTTP ${res.status}`, res.status, null)
          continue
        }
        return { res, attempts: attempt + 1 }
      } catch (e) {
        lastError = e
      }
    }
    const detail = lastError instanceof Error ? lastError.message : String(lastError)
    throw new R2Error(redact(`request to ${describeConfig(this.config)} failed after ${this.retries + 1} attempt(s): ${detail}`, this.config.credentials), 0, null)
  }

  private async fail(op: string, key: string, res: Response): Promise<never> {
    const { code, message } = await errorDetail(res)
    const text = redact(`${op} ${key} in ${describeConfig(this.config)}: HTTP ${res.status}${code ? ` ${code}` : ''}${message ? ` (${message})` : ''}`, this.config.credentials)
    if (res.status === 412) throw new PreconditionFailedError(text, res.status, code)
    throw new R2Error(text, res.status, code)
  }

  /** Upload `bytes` with `Content-Type: application/vnd.sqlite3` and the given metadata. */
  async put(key: string, bytes: Uint8Array, meta: Record<string, string>, opts: PutOptions = {}): Promise<{ etag: string | null }> {
    const { res, attempts } = await this.send(() => this.signPut(key, bytes, meta, opts))
    if (res.ok) {
      await res.body?.cancel()
      return { etag: res.headers.get('etag') }
    }
    // A create-only put that failed transiently may still have landed; the retry then sees
    // 412. If the object now there is byte-for-byte this upload, the put succeeded.
    if (res.status === 412 && attempts > 1) {
      const head = await this.head(key)
      const sameMeta = head !== null && Object.entries(meta).every(([k, v]) => head.meta[k] === v)
      if (head !== null && head.size === bytes.byteLength && sameMeta) {
        await res.body?.cancel()
        return { etag: head.etag }
      }
    }
    return this.fail('PUT', key, res)
  }

  /** The object's size, ETag and metadata, or null if there is no such key. */
  async head(key: string): Promise<ObjectHead | null> {
    const { res } = await this.send(() => this.signRequest('HEAD', key))
    if (res.status === 404) return null
    if (!res.ok) return this.fail('HEAD', key, res)
    return headFromResponse(key, res)
  }

  /** The object's bytes and head, or null if there is no such key. */
  async get(key: string): Promise<{ bytes: Uint8Array; head: ObjectHead } | null> {
    const { res } = await this.send(() => this.signRequest('GET', key))
    if (res.status === 404) {
      await res.body?.cancel()
      return null
    }
    if (!res.ok) return this.fail('GET', key, res)
    const head = headFromResponse(key, res)
    const bytes = new Uint8Array(await res.arrayBuffer())
    return { bytes, head: { ...head, size: bytes.byteLength } }
  }

  /** Delete a key. Deleting a missing key succeeds (S3 semantics). */
  async delete(key: string): Promise<void> {
    const { res } = await this.send(() => this.signRequest('DELETE', key))
    if (res.ok || res.status === 404) {
      await res.body?.cancel()
      return
    }
    return this.fail('DELETE', key, res)
  }

  /** Every key under `prefix`, following continuation tokens. */
  async list(prefix: string): Promise<ListEntry[]> {
    const client = new Bun.S3Client({
      accessKeyId: this.config.credentials.accessKeyId,
      secretAccessKey: this.config.credentials.secretAccessKey,
      bucket: this.config.bucket,
      endpoint: endpointOrigin(this.config.credentials.accountId),
      region: 'auto',
    })
    const entries: ListEntry[] = []
    let continuationToken: string | undefined
    do {
      let page: Awaited<ReturnType<typeof client.list>>
      try {
        page = await client.list({ prefix, maxKeys: 1000, ...(continuationToken ? { continuationToken } : {}) })
      } catch (e) {
        const detail = e instanceof Error ? `${(e as { code?: string }).code ?? e.name}: ${e.message}` : String(e)
        throw new R2Error(redact(`LIST ${prefix} in ${describeConfig(this.config)}: ${detail}`, this.config.credentials), 0, null)
      }
      for (const c of page.contents ?? []) {
        entries.push({ key: c.key, size: c.size ?? 0, lastModified: c.lastModified ?? null, etag: c.eTag ?? null })
      }
      continuationToken = page.isTruncated ? page.nextContinuationToken : undefined
    } while (continuationToken)
    return entries
  }
}
