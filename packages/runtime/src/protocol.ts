/**
 * Messages between the page and the database Worker.
 *
 * The Worker owns the database (OPFS sync access handles exist only in a dedicated
 * Worker), so everything the page wants from it is a request/response pair.
 */

/**
 * What `start` reports once the database is open.
 *
 * The schema heads are migration names such as `055_dataset_meta`. A database is safe to open
 * only while `schema_head` is not past `known_head`; a snapshot from a newer bundle has a
 * `schema_head` this bundle has never heard of.
 */
export interface StartInfo {
  /** The bundle version (the `forge-wasm` crate version). Same value as `bundle_version`. */
  version?: string
  bundle_version?: string
  migrations?: number
  /** Newest migration applied to the open database; null if it could not be read. */
  schema_head?: string | null
  /** Newest migration this bundle knows. */
  known_head?: string
  ms_install_storage?: number
  ms_open_and_migrate?: number
  /** A repeat `start`: reports the bundle's own versions, not the database's `schema_head`. */
  already_started?: boolean
}

/** What `exportDatabase` resolves to: the file, and what an upload must declare about it. */
export interface ExportInfo {
  /** The database as a SQLite file. */
  bytes: Uint8Array
  /** Newest migration recorded in `bytes`; null if they cannot be read as a database. */
  schema_head: string | null
  known_head: string
  bundle_version: string
}

export interface ImportInfo {
  imported_bytes: number
  migrations: number
  /** Newest migration in the database once the import has been brought up to date. */
  schema_head: string | null
  known_head: string
  bundle_version: string
}

/** Why `importDatabase` refuses a file; the page can switch on it. */
export type ImportRefusalCode = 'not_sqlite' | 'invalid_database' | 'schema_too_new'

/** The JSON `importDatabase` throws from the Worker for a refusal (the error `message`). */
export interface ImportRefusal {
  code: ImportRefusalCode
  message: string
  known_head: string
  /** The file's newest migration, for `schema_too_new`. */
  schema_head?: string
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
