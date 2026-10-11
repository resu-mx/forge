/**
 * R2 connection settings, read from the environment.
 *
 * Credentials come from `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY` (the
 * `just demo-data` R2 recipes load them from a 1Password Environment with `op run`). Error
 * messages name the variables, never their values, and `R2Credentials` redacts itself, so
 * logging a config object cannot print a credential.
 */

export type Target = 'preview' | 'prod'

export const TARGETS: readonly Target[] = ['preview', 'prod']

export const CREDENTIAL_VARS = ['R2_ACCOUNT_ID', 'R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY'] as const

/** The default bucket per target. */
export const DEFAULT_BUCKETS: Readonly<Record<Target, string>> = {
  prod: 'resumx-user-data',
  preview: 'resumx-user-data-preview',
}

/** The variable that overrides each target's bucket. */
export const BUCKET_VARS: Readonly<Record<Target, string>> = {
  prod: 'R2_BUCKET_PROD',
  preview: 'R2_BUCKET_PREVIEW',
}

export type Env = Record<string, string | undefined>

export class R2ConfigError extends Error {
  override name = 'R2ConfigError'
}

const REDACTED = '[redacted]'

/**
 * The secret lives in a private field behind a prototype getter, so it is absent from
 * spreads, `JSON.stringify`, `console.log` and `Bun.inspect` (which, unlike Node, prints
 * non-enumerable own properties).
 */
export class R2Credentials {
  readonly #secretAccessKey: string

  constructor(
    readonly accountId: string,
    readonly accessKeyId: string,
    secretAccessKey: string,
  ) {
    this.#secretAccessKey = secretAccessKey
  }

  get secretAccessKey(): string {
    return this.#secretAccessKey
  }

  toJSON(): Record<string, string> {
    return { accountId: REDACTED, accessKeyId: REDACTED, secretAccessKey: REDACTED }
  }

  toString(): string {
    return `R2Credentials { ${REDACTED} }`
  }

  [Symbol.for('nodejs.util.inspect.custom')](): string {
    return this.toString()
  }
}

export interface R2Config {
  target: Target
  bucket: string
  credentials: R2Credentials
}

const ACCOUNT_ID = /^[0-9a-f]{32}$/
// R2 bucket names: 3-63 characters, lowercase letters, digits and hyphens, no leading or
// trailing hyphen.
const BUCKET_NAME = /^[a-z0-9](?:[a-z0-9-]{1,61})[a-z0-9]$/

export function parseTarget(value: string | undefined): Target {
  if (value === 'preview' || value === 'prod') return value
  throw new R2ConfigError(`--target must be one of ${TARGETS.join(', ')} (got ${value === undefined ? 'nothing' : JSON.stringify(value)})`)
}

/** The bucket for a target: the override variable if set, else the default. */
export function bucketFor(target: Target, env: Env = process.env): string {
  const override = env[BUCKET_VARS[target]]
  const bucket = override === undefined || override === '' ? DEFAULT_BUCKETS[target] : override
  if (!BUCKET_NAME.test(bucket)) throw new R2ConfigError(`${BUCKET_VARS[target]} is not a valid R2 bucket name`)
  return bucket
}

/** Names of the credential variables that are unset or empty. */
export function missingCredentials(env: Env = process.env): string[] {
  return CREDENTIAL_VARS.filter((name) => !env[name])
}

export function hasCredentials(env: Env = process.env): boolean {
  return missingCredentials(env).length === 0
}

export function loadCredentials(env: Env = process.env): R2Credentials {
  const missing = missingCredentials(env)
  if (missing.length > 0) {
    throw new R2ConfigError(
      `missing ${missing.join(', ')}: run R2 commands through \`just demo-data <recipe>\` (it loads them with op run), or export them`,
    )
  }
  const accountId = env.R2_ACCOUNT_ID as string
  if (!ACCOUNT_ID.test(accountId)) throw new R2ConfigError('R2_ACCOUNT_ID is not a 32-character lowercase hex account id')
  return new R2Credentials(accountId, env.R2_ACCESS_KEY_ID as string, env.R2_SECRET_ACCESS_KEY as string)
}

export function loadR2Config(target: Target, env: Env = process.env): R2Config {
  return { target, bucket: bucketFor(target, env), credentials: loadCredentials(env) }
}

/** `https://<account>.r2.cloudflarestorage.com` (the S3 API endpoint). */
export function endpointOrigin(accountId: string): string {
  return `https://${accountId}.r2.cloudflarestorage.com`
}

/** Encode each path segment of an object key, keeping the `/` separators. */
export function encodeKey(key: string): string {
  return key.split('/').map(encodeURIComponent).join('/')
}

/** Path-style object URL: `https://<account>.r2.cloudflarestorage.com/<bucket>/<key>`. */
export function objectUrl(config: R2Config, key: string): string {
  return `${endpointOrigin(config.credentials.accountId)}/${config.bucket}/${encodeKey(key)}`
}

/** Throws unless `bucket` is a preview bucket (its name ends in `-preview`). */
export function assertPreviewBucket(bucket: string): void {
  if (!bucket.endsWith('-preview')) throw new R2ConfigError(`refusing to run against bucket ${bucket}: only a bucket whose name ends in -preview is allowed`)
}

/** `text` with any credential value (account id, access key id, secret) replaced. */
export function redact(text: string, credentials: R2Credentials): string {
  let out = text
  for (const [value, label] of [
    [credentials.secretAccessKey, '<R2_SECRET_ACCESS_KEY>'],
    [credentials.accessKeyId, '<R2_ACCESS_KEY_ID>'],
    [credentials.accountId, '<R2_ACCOUNT_ID>'],
  ] as const) {
    if (value) out = out.split(value).join(label)
  }
  return out
}

/** A printable description of where a config points, with no credential in it. */
export function describeConfig(config: R2Config): string {
  return `${config.target} bucket ${config.bucket}`
}
