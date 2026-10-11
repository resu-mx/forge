/**
 * Thin helpers around `ForgeClient`: unwrap `Result<T>` into values (throwing with context),
 * plus a raw-`fetch` client for the routes or fields the SDK does not cover.
 */

import type { Result } from '@forge/sdk'

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status?: number,
    readonly code?: string,
  ) {
    super(message)
  }
}

/** Await an SDK call and return its data, or throw naming what was being done. */
export async function unwrap<T>(call: Promise<Result<T>>, what: string): Promise<T> {
  const result = await call
  if (!result.ok) {
    throw new ApiError(`${what}: ${result.error.code} ${result.error.message}`, undefined, result.error.code)
  }
  return result.data
}

export type Raw = <T = unknown>(method: string, path: string, body?: unknown) => Promise<T>

/**
 * A JSON client for `<baseUrl>/api<path>`. Unwraps the `{ data }` envelope; an empty 2xx
 * body (204, or the contact link routes' empty 201) yields `undefined`.
 */
export function rawClient(baseUrl: string): Raw {
  return async <T>(method: string, path: string, body?: unknown): Promise<T> => {
    const res = await fetch(`${baseUrl}/api${path}`, {
      method,
      headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
    const text = await res.text()
    if (!res.ok) {
      let code: string | undefined
      let message = text
      try {
        const err = (JSON.parse(text) as { error?: { code?: string; message?: string } }).error
        code = err?.code
        message = err?.message ?? text
      } catch {
        // not JSON
      }
      throw new ApiError(`${method} ${path}: HTTP ${res.status} ${code ?? ''} ${message}`.trim(), res.status, code)
    }
    if (text.trim() === '') return undefined as T
    return (JSON.parse(text) as { data: T }).data
  }
}

/** Fetch a binary response (e.g. a PDF) and return its bytes. */
export async function fetchBytes(baseUrl: string, method: string, path: string): Promise<Uint8Array> {
  const res = await fetch(`${baseUrl}/api${path}`, { method })
  if (!res.ok) throw new ApiError(`${method} ${path}: HTTP ${res.status} ${await res.text()}`, res.status)
  return new Uint8Array(await res.arrayBuffer())
}
