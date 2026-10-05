/**
 * Golden fixture shared with the Rust port (forge-sdk `services::tagline`). Every case runs
 * through the TS implementation and must equal fixtures/tagline-golden.json. Regenerate with
 *   UPDATE_TAGLINE_GOLDEN=1 bun test src/services/__tests__/tagline-golden.test.ts
 */
import { describe, test, expect } from 'bun:test'
import { readdirSync, readFileSync, writeFileSync } from 'fs'
import { join } from 'path'
import { tokenize, computeTfIdf, rankKeywords, generateTagline } from '../tagline-service'
import type { Skill } from '../../types'

type Case =
  | { name: string; fn: 'tokenize'; text: string | null }
  | { name: string; fn: 'computeTfIdf'; texts: string[] }
  | { name: string; fn: 'rankKeywords'; scores: [string, number][]; skills: string[] }
  | { name: string; fn: 'generateTagline'; texts: string[]; skills: string[]; topK?: number; prefix?: string }
type CorpusCase = { name: string; files: string[]; skills: string[]; prefix: string | null }

const FIXTURE = join(import.meta.dir, 'fixtures', 'tagline-golden.json')
const CORPUS_DIR = join(import.meta.dir, '../../parser/__tests__/fixtures')
const skills = (names: string[]): Skill[] =>
  names.map((name) => ({ id: '00000000-0000-4000-8000-000000000000', name, category: 'other' }) as Skill)

const CASES: Case[] = [
  // -- the 27 tests in tagline-service.test.ts (one case per call) --
  { name: 'tokenize > lowercases input', fn: 'tokenize', text: 'Python AND Terraform' },
  { name: 'tokenize > drops stop words', fn: 'tokenize', text: 'The quick brown fox jumps over the lazy dog' },
  { name: 'tokenize > drops tokens under 2 characters', fn: 'tokenize', text: 'a b cd e' },
  { name: 'tokenize > strips punctuation around tokens', fn: 'tokenize', text: '"python," terraform; aws!' },
  { name: 'tokenize > preserves internal hyphens and plus signs', fn: 'tokenize', text: 'front-end c++ ci-cd' },
  { name: 'tokenize > drops pure numbers', fn: 'tokenize', text: '5 years of python 10 plus kubernetes' },
  { name: 'tokenize > empty input returns empty array', fn: 'tokenize', text: '' },
  { name: 'tokenize > empty input returns empty array', fn: 'tokenize', text: null },
  { name: 'tokenize > drops common job-description filler words', fn: 'tokenize', text: 'looking for strong experience working with teams' },
  { name: 'computeTfIdf > empty corpus yields empty map', fn: 'computeTfIdf', texts: [] },
  { name: 'computeTfIdf > single JD assigns positive scores to every term', fn: 'computeTfIdf', texts: ['python terraform aws'] },
  { name: 'computeTfIdf > term frequency in one JD boosts score', fn: 'computeTfIdf', texts: ['python python python terraform'] },
  { name: 'computeTfIdf > cross-JD repetition still accumulates score', fn: 'computeTfIdf', texts: ['python terraform', 'python kubernetes'] },
  { name: 'computeTfIdf > ubiquitous terms get down-weighted relative to unique terms', fn: 'computeTfIdf', texts: ['common rare1', 'common rare2', 'common rare3'] },
  { name: 'computeTfIdf > stop words are excluded from scores', fn: 'computeTfIdf', texts: ['the quick brown fox'] },
  { name: 'rankKeywords > returns entries sorted by score descending', fn: 'rankKeywords', scores: [['alpha', 1], ['beta', 3], ['gamma', 2]], skills: [] },
  { name: 'rankKeywords > skill match boosts score by SKILL_MATCH_BOOST', fn: 'rankKeywords', scores: [['python', 1], ['terraform', 1]], skills: ['Python'] },
  { name: 'rankKeywords > ties break alphabetically ascending', fn: 'rankKeywords', scores: [['zebra', 1], ['apple', 1], ['mango', 1]], skills: [] },
  { name: 'rankKeywords > empty scores yield empty ranking', fn: 'rankKeywords', scores: [], skills: [] },
  { name: 'generateTagline > empty corpus yields empty tagline', fn: 'generateTagline', texts: [], skills: [] },
  { name: 'generateTagline > whitespace-only texts yield empty tagline', fn: 'generateTagline', texts: ['', '  ', '\n\t'], skills: [] },
  {
    name: 'generateTagline > produces top-K keywords joined by " + "',
    fn: 'generateTagline',
    texts: [
      'We build cloud infrastructure using python terraform kubernetes aws. ' +
        'Python experience required. Terraform mandatory.',
    ],
    skills: [],
    topK: 3,
  },
  {
    name: 'generateTagline > prefix renders as "<prefix> -- <keywords>"',
    fn: 'generateTagline',
    texts: ['python terraform kubernetes'],
    skills: [],
    prefix: 'Senior Platform Engineer',
  },
  {
    name: 'generateTagline > default topK is DEFAULT_TOP_K',
    fn: 'generateTagline',
    texts: ['alpha beta gamma delta epsilon zeta eta theta'],
    skills: [],
  },
  {
    name: 'generateTagline > skill-matched terms rank above non-matched with equal base score',
    fn: 'generateTagline',
    texts: ['terraform postgres'],
    skills: ['Terraform'],
  },
  {
    name: 'generateTagline > handles multi-JD corpus and aggregates scores',
    fn: 'generateTagline',
    texts: ['python aws kafka streaming', 'python aws kinesis streaming', 'python gcp pubsub'],
    skills: [],
    topK: 3,
  },
  {
    name: 'generateTagline > matches skills case-insensitively',
    fn: 'generateTagline',
    texts: ['JavaScript typescript'],
    skills: ['TypeScript', 'JAVASCRIPT'],
    topK: 2,
  },
  {
    name: 'generateTagline > omitting prefix yields bare keyword list',
    fn: 'generateTagline',
    texts: ['alpha beta gamma'],
    skills: [],
    topK: 3,
  },
  // -- issue examples and probes --
  { name: 'issue#67 > tokenize example', fn: 'tokenize', text: 'Front-end (React/TypeScript), C++ and CI-CD!' },
  {
    name: 'issue#67 > multi-JD example',
    fn: 'generateTagline',
    texts: ['kubernetes terraform python', 'kubernetes prometheus grafana'],
    skills: [],
    prefix: 'SRE',
  },
  {
    name: 'issue#69 > regenerate example',
    fn: 'generateTagline',
    texts: ['Terraform Kubernetes Python Ansible'],
    skills: ['Terraform'],
    prefix: 'Senior Platform Engineer',
  },
  {
    name: 'probe > tie order, punctuation',
    fn: 'rankKeywords',
    scores: [['ca', 1], ['c0', 1], ['c++', 1], ['c#', 1], ['c-d', 1], ['c-', 1], ['c', 1]],
    skills: [],
  },
  {
    name: 'probe > tie order, non-ASCII',
    fn: 'rankKeywords',
    scores: [['west', 1], ['we’re', 1], ['we-re', 1], ['naivf', 1], ['naïve', 1], ['naive', 1], ['company’s', 1]],
    skills: [],
  },
  { name: 'probe > JS whitespace', fn: 'tokenize', text: 'alpha﻿beta gamma\u0085delta x yz' },
  { name: 'probe > trailing non-ASCII is stripped', fn: 'tokenize', text: 'café résumé naïve über' },
  { name: 'probe > prefix empty string is omitted', fn: 'generateTagline', texts: ['alpha beta'], skills: [], prefix: '' },
  { name: 'probe > U+FEFF-only text is blank', fn: 'generateTagline', texts: ['﻿', '  '], skills: [] },
  { name: 'probe > multi-word skill never matches', fn: 'generateTagline', texts: ['machine learning pipelines'], skills: ['Machine Learning'] },
]

const CORPUS: CorpusCase[] = [
  ...readdirSync(CORPUS_DIR)
    .filter((f) => f.endsWith('.txt'))
    .sort()
    .map((f) => ({ name: `corpus > ${f}`, files: [f], skills: [], prefix: null })),
  {
    name: 'corpus > platform mix',
    files: ['anthropic-agent-infra.txt', 'snorkel-training-infra.txt', 'capital-one-swe.txt'],
    skills: ['Python', 'Kubernetes', 'AWS', 'Go'],
    prefix: 'Senior Platform Engineer',
  },
  {
    name: 'corpus > security pair',
    files: ['anthropic-cybersec-re.txt', 'betterup-ai-security.txt'],
    skills: ['Python', 'Rust'],
    prefix: 'Security Engineer',
  },
  {
    name: 'corpus > all JDs, no skills',
    files: readdirSync(CORPUS_DIR).filter((f) => f.endsWith('.txt')).sort(),
    skills: [],
    prefix: null,
  },
]

function run(c: Case): unknown {
  switch (c.fn) {
    case 'tokenize':
      return tokenize(c.text as string)
    case 'computeTfIdf':
      return [...computeTfIdf(c.texts).entries()]
    case 'rankKeywords':
      return rankKeywords(new Map(c.scores), skills(c.skills))
    case 'generateTagline':
      return generateTagline(c.texts, skills(c.skills), { topK: c.topK, prefix: c.prefix })
  }
}

function runCorpus(c: CorpusCase) {
  const texts = c.files.map((f) => readFileSync(join(CORPUS_DIR, f), 'utf-8'))
  return generateTagline(texts, skills(c.skills), { prefix: c.prefix ?? undefined })
}

describe('tagline golden fixture (shared with forge-sdk services::tagline)', () => {
  // JSON round-trip so undefined keys drop exactly as they do in the written fixture.
  const actual = JSON.parse(
    JSON.stringify({
      cases: CASES.map((c) => ({ ...c, expected: run(c) })),
      corpus: CORPUS.map((c) => ({ ...c, expected: runCorpus(c) })),
    }),
  )
  if (process.env.UPDATE_TAGLINE_GOLDEN) {
    writeFileSync(FIXTURE, JSON.stringify(actual, null, 1) + '\n')
  }

  test('the fixture matches the TS implementation', () => {
    expect(JSON.parse(readFileSync(FIXTURE, 'utf-8'))).toEqual(actual)
  })
})
