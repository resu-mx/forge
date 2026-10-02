/**
 * Messages between the page and the database Worker.
 *
 * The Worker owns the database (OPFS sync access handles exist only in a dedicated
 * Worker), so everything the page wants from it is a request/response pair.
 */

/** What `start` reports once the database is open. */
export interface StartInfo {
  version?: string
  migrations?: number
  ms_install_storage?: number
  ms_open_and_migrate?: number
  already_started?: boolean
}

export interface ImportInfo {
  imported_bytes: number
  migrations: number
}

/** `[name, value]` pairs, as the Rust side expects. */
export type HeaderPairs = [string, string][]

export interface RawResponse {
  status: number
  headers: HeaderPairs
  body: Uint8Array
}

export type WorkerRequest =
  | { id: number; op: 'start' }
  | { id: number; op: 'request'; method: string; path: string; headers: HeaderPairs; body: Uint8Array }
  | { id: number; op: 'export' }
  | { id: number; op: 'import'; bytes: Uint8Array }

export type WorkerReply =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: string }

/** The Typst compiler Worker (a separate, lazily loaded module; see typst-worker.ts). */
export interface PdfRequest {
  id: number
  op: 'compile'
  source: string
}

export type PdfReply =
  | { id: number; ok: true; result: Uint8Array }
  /** `error` is the JSON `{ message, details }` forge-typst throws, or a plain message. */
  | { id: number; ok: false; error: string }
