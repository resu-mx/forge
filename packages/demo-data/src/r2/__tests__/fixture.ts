/**
 * Small dataset files for the R2 tests: the real migrations applied to a fresh file, with or
 * without the `dataset_meta` marker, compacted (DELETE mode) or left in WAL mode, plus a
 * manifest describing the result.
 */

import { Database } from 'bun:sqlite'
import { copyFileSync, readFileSync, rmSync } from 'fs'
import { join } from 'path'
import { migratedDb } from '../../__tests__/helpers'
import { compact } from '../../postpass/compact'
import type { Manifest } from '../../postpass/manifest'
import { type DatasetMeta, schemaHead, writeDatasetMeta } from '../../postpass/meta'
import { sha256Hex } from '../objects'

export interface DatasetFixture {
  path: string
  bytes: Uint8Array
  manifest: Manifest
}

export interface VariantOptions {
  uuid: string
  persona: string
  /** `dataset_meta` overrides; `null` writes no marker at all (a plain migrated database). */
  meta?: Partial<DatasetMeta> | null
  /** Leave the file in WAL mode instead of compacting it. */
  wal?: boolean
}

/** Builds a migrated base once per directory, then cheap variants of it. */
export class FixtureFactory {
  private readonly base: string
  private n = 0

  constructor(private readonly dir: string) {
    this.base = join(dir, 'base.sqlite')
    const db = migratedDb(this.base)
    db.close()
    for (const suffix of ['-wal', '-shm']) rmSync(this.base + suffix, { force: true })
  }

  variant(opts: VariantOptions): DatasetFixture {
    const path = join(this.dir, `variant-${++this.n}.sqlite`)
    copyFileSync(this.base, path)
    const db = new Database(path)
    let head: string
    try {
      head = schemaHead(db)
      if (opts.meta !== null) {
        writeDatasetMeta(db, {
          kind: 'generated',
          generator: '@forge/demo-data',
          generator_version: '0.1.0',
          seed: 'test-seed',
          persona: opts.persona,
          dataset_uuid: opts.uuid,
          as_of: '2026-09-30T17:00:00Z',
          generated_at: '2026-10-01T00:00:00Z',
          schema_head: head,
          ...opts.meta,
        })
      }
    } finally {
      db.close()
    }
    if (!opts.wal) compact(path)
    const bytes = new Uint8Array(readFileSync(path))
    const manifest: Manifest = {
      uuid: opts.uuid,
      persona: opts.persona,
      seed: 'test-seed',
      as_of: '2026-09-30T17:00:00Z',
      generator: '@forge/demo-data',
      generator_version: '0.1.0',
      schema_head: opts.meta?.schema_head ?? head,
      generated_at: '2026-10-01T00:00:00Z',
      r2_key: `user/${opts.uuid}/data.sqlite`,
      bytes: bytes.byteLength,
      sha256: sha256Hex(bytes),
      content_fingerprint: 'c'.repeat(64),
      counts: {},
      coverage: {},
      overlay: [],
    }
    return { path, bytes, manifest }
  }
}
