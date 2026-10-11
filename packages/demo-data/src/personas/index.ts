/**
 * Persona registry. Each persona is a typed corpus under `personas/<slug>/`.
 */

import type { PersonaCorpus } from '../corpus/types'
import { earlyCareerDeveloper } from './early-career-developer'

export const PERSONAS: Readonly<Record<string, PersonaCorpus>> = {
  [earlyCareerDeveloper.slug]: earlyCareerDeveloper,
}

export function getPersona(slug: string): PersonaCorpus {
  const persona = PERSONAS[slug]
  if (!persona) throw new Error(`unknown persona "${slug}"; known: ${Object.keys(PERSONAS).join(', ')}`)
  return persona
}

/** `all` → every persona; otherwise the named one. */
export function selectPersonas(arg: string | undefined): PersonaCorpus[] {
  if (!arg || arg === 'all') return Object.values(PERSONAS)
  return [getPersona(arg)]
}
