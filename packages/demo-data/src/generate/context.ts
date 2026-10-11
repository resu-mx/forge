/**
 * State shared by the generation phases.
 */

import type { ForgeClient } from '@forge/sdk'
import type { Raw } from '../api'
import type { PersonaCorpus, RelDate } from '../corpus/types'
import type { Ledger } from '../ledger'
import type { NoteReferenceStep, OverlayStep } from '../postpass/overlay'
import { type Clock, monthsAgoDate } from '../time'

export type { NoteReferenceStep, OverlayStep }

export interface GenContext {
  corpus: PersonaCorpus
  forge: ForgeClient
  raw: Raw
  baseUrl: string
  ledger: Ledger
  clock: Clock
  seed: string
  overlay: OverlayStep[]
  /** Note references the API refused, inserted by the post-pass. */
  noteReferences: NoteReferenceStep[]
  /** Seeded reference rows, by name. */
  seeded: {
    archetypes: Map<string, string>
    domains: Map<string, string>
    templates: Map<string, string>
  }
  log: (message: string) => void
}

/** Resolve a relative calendar date against `--as-of`. */
export function relDate(ctx: GenContext, rel: RelDate | undefined): string | undefined {
  return rel === undefined ? undefined : monthsAgoDate(ctx.clock.asOf, rel.monthsAgo, rel.day ?? 1)
}

/** A timestamp `daysAgo` before `--as-of` for `<kind>:<key>`, optionally after another one. */
export function stamp(ctx: GenContext, daysAgo: number, stream: string, after?: string): string {
  return ctx.clock.at(daysAgo, stream, after)
}

/** The skill's name for a skill key (technologies are linked by name). */
export function skillName(ctx: GenContext, key: string): string {
  const skill = ctx.corpus.skills.find((s) => s.key === key)
  if (!skill) throw new Error(`unknown skill key "${key}"`)
  return skill.name
}

/** A one-line progress logger. */
export function phase(ctx: GenContext, name: string): void {
  ctx.log(`  · ${name}`)
}
