/**
 * Persona `early-career-developer`: a recent computer science graduate in Columbus, OH,
 * studying part-time for a master's and applying for platform, solutions and junior ML roles.
 *
 * Coverage it is built for: all four education types (degree with campus and GPA, an
 * in-progress master's, a completion certificate, a course, self-taught study), an internship,
 * a TA job and a part-time help desk role, a capstone and a hackathon project, a lightning
 * talk and a poster, every source type, an in-progress certification, a driver's license,
 * and every board column for sources, bullets, perspectives, resumes, JDs and organizations.
 */

import type { PersonaCorpus } from '../../corpus/types'
import { contacts, jds, notes, resumes, summaries } from './applications'
import { bullets, perspectives, sources } from './career'
import { answers, certifications, credentials, industries, orgs, profile, roleTypes, skills } from './reference'

export const earlyCareerDeveloper: PersonaCorpus = {
  slug: 'early-career-developer',
  summary:
    'Avery Lindqvist, a recent computer science graduate in Columbus, OH, studying part-time for an M.S. in Software Engineering and applying for associate platform, solutions and junior ML roles.',
  accountAgeDays: 420,
  profile,
  answers,
  industries,
  roleTypes,
  skills,
  orgs,
  credentials,
  certifications,
  sources,
  bullets,
  perspectives,
  summaries,
  jds,
  contacts,
  resumes,
  notes,
  dir: import.meta.dir,
}
