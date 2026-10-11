/**
 * Phase 11: notes referencing every note entity type, plus the "About this demo dataset"
 * note that tells anyone who opens the file what it is.
 */

import { ApiError, unwrap } from '../api'
import type { NoteEntityType, PersonaCorpus } from '../corpus/types'
import { personaUuid } from '../ids'
import { type GenContext, phase, stamp } from './context'

export const ABOUT_NOTE_KEY = 'about-this-dataset'
export const ABOUT_NOTE_TITLE = 'About this demo dataset'

/** The ledger kind behind each note entity type. */
const REF_KIND: Record<NoteEntityType, string> = {
  source: 'source',
  bullet: 'bullet',
  perspective: 'perspective',
  resume_entry: 'resume_entry',
  resume: 'resume',
  skill: 'skill',
  organization: 'org',
  job_description: 'jd',
  contact: 'contact',
  credential: 'credential',
  certification: 'certification',
}

/** Deterministic text: no wall-clock time, so the content fingerprint is stable. */
export function aboutNoteContent(corpus: PersonaCorpus, generatorVersion: string): string {
  return [
    `This workspace is a generated demo dataset (persona "${corpus.slug}", dataset ${personaUuid(corpus.slug)}).`,
    `It was produced by @forge/demo-data ${generatorVersion} through the Forge API, so it exercises the same flows a real user would.`,
    'Every person, organization, email address, phone number and URL in it is fictional. Cities and states are real so the map views work.',
    'Some statuses (archived bullets and perspectives, source review states) were set directly because no screen can set them yet.',
    `Persona: ${corpus.summary}`,
  ].join('\n\n')
}

export async function phaseNotes(ctx: GenContext, generatorVersion: string): Promise<void> {
  phase(ctx, 'notes')
  const { corpus, forge, ledger } = ctx

  const about = await unwrap(
    forge.notes.create({ title: ABOUT_NOTE_TITLE, content: aboutNoteContent(corpus, generatorVersion) }),
    'about note',
  )
  // Newest note, so it sorts first in the notes list.
  const aboutTime = stamp(ctx, 0, `note:${ABOUT_NOTE_KEY}`)
  ledger.add('note', ABOUT_NOTE_KEY, 'user_notes', about.id, { created_at: aboutTime, updated_at: aboutTime })

  for (const n of corpus.notes) {
    const row = await unwrap(forge.notes.create({ title: n.title, content: n.content }), `note ${n.key}`)
    const t = stamp(ctx, n.daysAgo, `note:${n.key}`)
    ledger.add('note', n.key, 'user_notes', row.id, { created_at: t, updated_at: t })
    for (const ref of n.refs) {
      const entityId = ledger.id(REF_KIND[ref.type], ref.key)
      try {
        await unwrap(forge.notes.addReference(row.id, { entity_type: ref.type, entity_id: entityId }), `note ${n.key} → ${ref.type} ${ref.key}`)
      } catch (e) {
        // The Rust API's NoteReferenceEntityType has no credential/certification (the schema
        // and the TS API do), so those references are inserted by the post-pass instead.
        const refused = e instanceof ApiError && e.code === 'VALIDATION_ERROR' && /Invalid entity_type/.test(e.message)
        if (!refused || (ref.type !== 'credential' && ref.type !== 'certification')) throw e
        ctx.noteReferences.push({ ref: `note:${n.key}`, noteId: row.id, entityType: ref.type, entityRef: `${ref.type}:${ref.key}`, entityId })
      }
    }
  }
}
