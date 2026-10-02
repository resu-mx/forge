import type {
  DataExportBundle,
  ForgeError,
  RequestFn,
  ResumeDocument,
  Result,
} from '../types'

type FetchFn = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>

export class ExportResource {
  constructor(
    private request: RequestFn,
    private baseUrl: string,
    private fetchImpl: FetchFn = (input, init) => globalThis.fetch(input, init),
  ) {}

  /** Export a resume as JSON (returns the full IR document). */
  async resumeAsJson(id: string): Promise<Result<ResumeDocument>> {
    return this.request<ResumeDocument>(
      'GET',
      `/api/export/resume/${id}?format=json`,
    )
  }

  /**
   * Download a resume as a binary blob (PDF, Markdown, or LaTeX).
   *
   * Uses raw `fetch()` instead of `this.request()` because the response
   * is not a JSON envelope — it is the raw file content.
   */
  async downloadResume(
    id: string,
    format: 'pdf' | 'markdown' | 'latex',
  ): Promise<Result<Blob>> {
    try {
      const response = await this.fetchImpl(
        `${this.baseUrl}/api/export/resume/${id}?format=${format}`,
      )

      if (!response.ok) {
        // Try to parse JSON error envelope
        try {
          const json = await response.json() as { error?: ForgeError }
          const error = json.error ?? {
            code: 'EXPORT_FAILED',
            message: `HTTP ${response.status}`,
          }
          return { ok: false, error }
        } catch {
          return {
            ok: false,
            error: {
              code: 'EXPORT_FAILED',
              message: `HTTP ${response.status}: ${response.statusText}`,
            },
          }
        }
      }

      const blob = await response.blob()
      return { ok: true, data: blob }
    } catch (err) {
      return {
        ok: false,
        error: { code: 'NETWORK_ERROR', message: String(err) },
      }
    }
  }

  /** Export entity data as a JSON bundle. */
  async exportData(entities: string[]): Promise<Result<DataExportBundle>> {
    return this.request<DataExportBundle>(
      'GET',
      `/api/export/data?entities=${entities.join(',')}`,
    )
  }

  /**
   * Download a full database dump as SQL text.
   *
   * Uses raw `fetch()` because the response is SQL text, not JSON.
   */
  async dumpDatabase(): Promise<Result<Blob>> {
    try {
      const response = await this.fetchImpl(`${this.baseUrl}/api/export/dump`)

      if (!response.ok) {
        try {
          const json = await response.json() as { error?: ForgeError }
          const error = json.error ?? {
            code: 'DUMP_FAILED',
            message: `HTTP ${response.status}`,
          }
          return { ok: false, error }
        } catch {
          return {
            ok: false,
            error: {
              code: 'DUMP_FAILED',
              message: `HTTP ${response.status}: ${response.statusText}`,
            },
          }
        }
      }

      const blob = await response.blob()
      return { ok: true, data: blob }
    } catch (err) {
      return {
        ok: false,
        error: { code: 'NETWORK_ERROR', message: String(err) },
      }
    }
  }
}
