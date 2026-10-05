import type { ForgeError } from '@forge/sdk'

/**
 * The error carried by a failed raw `fetch('/api/…')` response, read from the API's
 * `{"error":{"code","message"}}` envelope. For calls the SDK does not cover yet (resu-mx/forge#127).
 */
export async function responseError(res: Response): Promise<ForgeError> {
  try {
    const body = (await res.json()) as { error?: { code?: unknown; message?: unknown } }
    const { code, message } = body?.error ?? {}
    if (typeof code === 'string' && typeof message === 'string') return { code, message }
  } catch {
    // Not JSON. Don't say "non-JSON" below: friendlyError maps that to "server unreachable".
  }
  return { code: 'UNKNOWN_ERROR', message: `HTTP ${res.status}` }
}
