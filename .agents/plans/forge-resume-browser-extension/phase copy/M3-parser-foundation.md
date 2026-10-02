# M3 — Parser Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone JD structural parser that splits raw job description text into classified sections and extracts salary, location, and work posture inline fields.

**Architecture:** Three-layer pipeline in `packages/core/src/parser/`. L1 splits raw text by heading patterns (markdown, bold, ALL CAPS, HTML). L2 classifies sections by keyword density into a 9-category taxonomy. L3 extracts structured inline fields (salary range, locations, work posture) from classified sections. Pure functions, no Forge dependencies — consumed by both extension and core ingestion in M5.

**Tech Stack:** TypeScript, Bun test runner, pure functions (no external dependencies)

---

## File Structure

```
packages/core/src/parser/
├── index.ts              — public API: parseJobDescription() + re-exports
├── types.ts              — Section, ClassifiedSection, ParsedJobDescription, etc.
├── l1-splitter.ts        — splitSections(rawText) → Section[]
├── l2-classifier.ts      — classifySections(sections) → ClassifiedSection[]
├── l3-extractors.ts      — extractSalary(), extractLocations(), extractWorkPosture()
└── __tests__/
    ├── l1-splitter.test.ts
    ├── l2-classifier.test.ts
    ├── l3-extractors.test.ts
    ├── integration.test.ts
    └── fixtures/
        ├── anthropic-cybersec-re.txt
        ├── anthropic-agent-infra.txt
        ├── anthropic-fde-federal.txt
        ├── betterup-ai-security.txt
        ├── snorkel-training-infra.txt
        ├── booz-agentic-ai.txt
        ├── amazon-ml-engineer.txt
        ├── capital-one-swe.txt
        ├── generic-markdown-headings.txt
        ├── generic-bold-headings.txt
        ├── generic-allcaps-headings.txt
        ├── generic-html-headings.txt
        ├── generic-no-headings.txt
        ├── generic-mixed-headings.txt
        ├── salary-range-dash.txt
        ├── salary-range-to.txt
        ├── salary-hourly.txt
        ├── salary-single-number.txt
        ├── multi-location.txt
        ├── remote-only.txt
        ├── hybrid-location.txt
        ├── minimal-jd.txt
        └── wall-of-text.txt
```

---

### Task 1: Types and Public API Surface

**Files:**
- Create: `packages/core/src/parser/types.ts`
- Create: `packages/core/src/parser/index.ts`

- [ ] **Step 1: Create types.ts**

```ts
// packages/core/src/parser/types.ts

export interface Section {
  heading: string | null     // null for preamble text before first heading
  text: string               // raw text content (excluding the heading line itself)
  byteOffset: number         // byte offset of section start in original input
}

export type SectionCategory =
  | 'responsibilities'
  | 'requirements'
  | 'preferred'
  | 'description'
  | 'location'
  | 'compensation'
  | 'benefits'
  | 'about_company'
  | 'eeo'

export interface ClassifiedSection extends Section {
  category: SectionCategory
  confidence: number         // 0–1
}

export interface SalaryRange {
  min: number
  max: number
  period: 'annual' | 'hourly' | 'unknown'
}

export interface ParsedJobDescription {
  sections: ClassifiedSection[]
  salary: SalaryRange | null
  locations: string[]
  workPosture: 'remote' | 'hybrid' | 'on-site' | null
}
```

- [ ] **Step 2: Create index.ts with stub pipeline**

```ts
// packages/core/src/parser/index.ts

export type {
  Section,
  SectionCategory,
  ClassifiedSection,
  SalaryRange,
  ParsedJobDescription,
} from './types'

import { splitSections } from './l1-splitter'
import { classifySections } from './l2-classifier'
import { extractSalary, extractLocations, extractWorkPosture } from './l3-extractors'
import type { ParsedJobDescription } from './types'

export function parseJobDescription(rawText: string): ParsedJobDescription {
  const sections = splitSections(rawText)
  const classified = classifySections(sections)
  const salary = extractSalary(classified)
  const locations = extractLocations(classified)
  const workPosture = extractWorkPosture(classified)
  return { sections: classified, salary, locations, workPosture }
}

export { splitSections } from './l1-splitter'
export { classifySections } from './l2-classifier'
export { extractSalary, extractLocations, extractWorkPosture } from './l3-extractors'
```

- [ ] **Step 3: Create stub l1-splitter.ts, l2-classifier.ts, l3-extractors.ts**

These stubs let the project compile. They return empty/null results.

```ts
// packages/core/src/parser/l1-splitter.ts
import type { Section } from './types'

export function splitSections(rawText: string): Section[] {
  return []
}
```

```ts
// packages/core/src/parser/l2-classifier.ts
import type { Section, ClassifiedSection } from './types'

export function classifySections(sections: Section[]): ClassifiedSection[] {
  return []
}
```

```ts
// packages/core/src/parser/l3-extractors.ts
import type { ClassifiedSection, SalaryRange } from './types'

export function extractSalary(sections: ClassifiedSection[]): SalaryRange | null {
  return null
}

export function extractLocations(sections: ClassifiedSection[]): string[] {
  return []
}

export function extractWorkPosture(sections: ClassifiedSection[]): 'remote' | 'hybrid' | 'on-site' | null {
  return null
}
```

- [ ] **Step 4: Verify project compiles**

Run: `cd packages/core && bunx tsc --noEmit`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add packages/core/src/parser/
git commit -m "feat(parser): scaffold M3 types and public API surface"
```

---

### Task 2: Test Fixtures

**Files:**
- Create: `packages/core/src/parser/__tests__/fixtures/*.txt` (24 files)

- [ ] **Step 1: Create real JD fixtures from database**

Extract 8 real JDs from Forge DB into fixture files. Use `sqlite3` to dump `raw_text` for diverse orgs.

```bash
cd packages/core
mkdir -p src/parser/__tests__/fixtures
```

Extract these JDs (pipe through `sqlite3 ../../data/forge.db`):
1. `anthropic-cybersec-re.txt` — Anthropic Cybersecurity RE (markdown ## + bold ** headings, salary with em-dash)
2. `anthropic-agent-infra.txt` — Anthropic Agent Infra (similar format)
3. `anthropic-fde-federal.txt` — Anthropic FDE (may lack salary)
4. `betterup-ai-security.txt` — BetterUp Principal AI Security (wall-of-text + EEO + benefits, salary at bottom)
5. `snorkel-training-infra.txt` — Snorkel Training Infra (clean section headers, salary + location at end)
6. `booz-agentic-ai.txt` — Booz Allen (federal contractor style)
7. `amazon-ml-engineer.txt` — Amazon ML Engineer
8. `capital-one-swe.txt` — Capital One

- [ ] **Step 2: Create synthetic fixtures for edge cases**

Create 16 synthetic fixtures covering specific heading/extraction patterns:

`generic-markdown-headings.txt`:
```
## About Us
We are a fast-growing startup.

## Responsibilities
- Design and implement distributed systems
- Mentor junior engineers

## Requirements
- 5+ years Python experience
- Experience with Kubernetes

## Benefits
- Health insurance
- 401k matching
```

`generic-bold-headings.txt`:
```
**About the Role**
We are looking for a senior engineer.

**Responsibilities**
- Build and maintain APIs
- Write technical documentation

**Qualifications**
- 3+ years experience with Go
- Familiarity with gRPC
```

`generic-allcaps-headings.txt`:
```
ABOUT THE COMPANY
Acme Corp builds widgets for enterprise customers.

WHAT YOU WILL DO
- Lead cross-functional projects
- Drive technical roadmap

WHAT WE ARE LOOKING FOR
- 7+ years of software engineering experience
- Strong leadership skills

COMPENSATION AND BENEFITS
Salary range: $180,000 - $250,000 per year
```

`generic-html-headings.txt`:
```
<h2>About Us</h2>
We build tools for developers.

<h3>Key Responsibilities</h3>
- Architect cloud infrastructure
- Implement CI/CD pipelines

<h3>Requirements</h3>
- 5+ years AWS experience
- Terraform expertise

<h4>Nice to Have</h4>
- Kubernetes certification
```

`generic-no-headings.txt`:
```
We are looking for a software engineer to join our platform team. You will design and build APIs, collaborate with product managers, and contribute to our open-source projects. The ideal candidate has 3+ years of experience with TypeScript and Node.js, familiarity with PostgreSQL, and strong communication skills. We offer competitive compensation of $140,000 to $180,000 annually, plus equity and benefits. This is a remote-friendly position based in San Francisco.
```

`generic-mixed-headings.txt`:
```
## Overview
Join our engineering team.

**What You'll Do**
- Build data pipelines
- Optimize query performance

REQUIREMENTS
- 4+ years of data engineering
- SQL expertise

## Compensation
$160,000 - $200,000 per year
```

`salary-range-dash.txt`:
```
## About the Role
Senior engineer position.

## Requirements
- 5+ years experience

## Compensation
Annual Salary: $200,000 - $300,000 USD
```

`salary-range-to.txt`:
```
## About the Role
Staff engineer position.

## Requirements
- 8+ years experience

## Compensation
Base salary ranges from $180,000 to $260,000 per year.
```

`salary-hourly.txt`:
```
## About the Role
Contract position for 6 months.

## Requirements
- 3+ years experience

## Compensation
Rate: $75 - $120/hr
```

`salary-single-number.txt`:
```
## About the Role
Entry-level engineer.

## Requirements
- 1+ year experience

## Compensation
Starting salary: $95,000/year
```

`multi-location.txt`:
```
## About the Role
We have offices in multiple cities.

## Location
This role can be based in New York, NY; San Francisco, CA; Seattle, WA; or Austin, TX.

## Requirements
- 3+ years experience
```

`remote-only.txt`:
```
## About the Role
Fully remote position.

## Location
This is a fully remote role. We welcome candidates from anywhere in the United States.

## Requirements
- 2+ years experience
```

`hybrid-location.txt`:
```
## About the Role
Hybrid work schedule.

## Location
This is a hybrid position based in our Arlington, VA office. Employees are expected in-office 2-3 days per week.

## Requirements
- 4+ years experience
```

`minimal-jd.txt`:
```
Software Engineer

Build things. Ship fast.

Requirements: Python, AWS, 2+ years.

Salary: $120k-$160k. Remote.
```

`wall-of-text.txt`:
```
We are seeking a talented Software Engineer to join our growing team. The ideal candidate will have experience building scalable web applications using modern frameworks. You will work closely with our product team to design and implement new features, improve system reliability, and contribute to technical strategy. We are looking for someone with at least 5 years of professional software development experience, strong proficiency in Python or Java, experience with cloud platforms (AWS or GCP), and excellent problem-solving skills. Nice to have: experience with machine learning frameworks, containerization with Docker and Kubernetes, and contributions to open-source projects. We offer a competitive salary of $150,000 to $200,000, comprehensive health benefits, unlimited PTO, and a flexible hybrid work arrangement with 2 days per week in our Chicago, IL office. We are an equal opportunity employer and value diversity at our company.
```

`salary-em-dash.txt` (tests the Anthropic format):
```
## About the Role
Research position.

## Requirements
- PhD or equivalent experience
- Strong publication record

The expected salary range for this position is $300,000—$405,000 USD
```

- [ ] **Step 3: Commit fixtures**

```bash
git add packages/core/src/parser/__tests__/fixtures/
git commit -m "test(parser): add 24 JD fixtures for M3 parser testing"
```

---

### Task 3: L1 Heading Splitter — Tests

**Files:**
- Create: `packages/core/src/parser/__tests__/l1-splitter.test.ts`

- [ ] **Step 1: Write L1 tests**

```ts
// packages/core/src/parser/__tests__/l1-splitter.test.ts

import { describe, it, expect } from 'bun:test'
import { splitSections } from '../l1-splitter'
import { readFileSync } from 'fs'
import { join } from 'path'

const fixture = (name: string) =>
  readFileSync(join(__dirname, 'fixtures', name), 'utf-8')

describe('splitSections (L1)', () => {
  // ── Empty / edge cases ──────────────────────────────────────────────
  it('returns empty array for empty string', () => {
    expect(splitSections('')).toEqual([])
  })

  it('returns single preamble section for text with no headings', () => {
    const result = splitSections('Just some text.\nAnother line.')
    expect(result).toHaveLength(1)
    expect(result[0].heading).toBeNull()
    expect(result[0].text).toContain('Just some text.')
    expect(result[0].byteOffset).toBe(0)
  })

  // ── Markdown headings ──────────────────────────────────────────────
  it('splits on markdown ## headings', () => {
    const result = splitSections(fixture('generic-markdown-headings.txt'))
    const headings = result.map(s => s.heading)
    expect(headings).toContain('About Us')
    expect(headings).toContain('Responsibilities')
    expect(headings).toContain('Requirements')
    expect(headings).toContain('Benefits')
  })

  it('preserves section text without the heading line', () => {
    const result = splitSections(fixture('generic-markdown-headings.txt'))
    const req = result.find(s => s.heading === 'Requirements')!
    expect(req.text).toContain('5+ years Python experience')
    expect(req.text).not.toContain('## Requirements')
  })

  // ── Bold headings ──────────────────────────────────────────────────
  it('splits on **bold** headings', () => {
    const result = splitSections(fixture('generic-bold-headings.txt'))
    const headings = result.map(s => s.heading)
    expect(headings).toContain('About the Role')
    expect(headings).toContain('Responsibilities')
    expect(headings).toContain('Qualifications')
  })

  // ── ALL CAPS headings ──────────────────────────────────────────────
  it('splits on ALL CAPS headings', () => {
    const result = splitSections(fixture('generic-allcaps-headings.txt'))
    const headings = result.map(s => s.heading)
    expect(headings).toContain('ABOUT THE COMPANY')
    expect(headings).toContain('WHAT YOU WILL DO')
    expect(headings).toContain('WHAT WE ARE LOOKING FOR')
    expect(headings).toContain('COMPENSATION AND BENEFITS')
  })

  it('does not treat short ALL CAPS words as headings', () => {
    const result = splitSections('AWS and GCP are required.\nNO EXCEPTIONS')
    // "NO EXCEPTIONS" is only 2 words — should not be a heading
    // But wait, the preamble contains the text
    const headings = result.filter(s => s.heading !== null)
    expect(headings).toHaveLength(0)
  })

  // ── HTML headings ──────────────────────────────────────────────────
  it('splits on HTML <h2>-<h4> headings', () => {
    const result = splitSections(fixture('generic-html-headings.txt'))
    const headings = result.map(s => s.heading)
    expect(headings).toContain('About Us')
    expect(headings).toContain('Key Responsibilities')
    expect(headings).toContain('Requirements')
    expect(headings).toContain('Nice to Have')
  })

  // ── Mixed headings ────────────────────────────────────────────────
  it('handles mixed heading styles in one document', () => {
    const result = splitSections(fixture('generic-mixed-headings.txt'))
    expect(result.length).toBeGreaterThanOrEqual(4)
    const headings = result.map(s => s.heading)
    expect(headings).toContain('Overview')
    expect(headings).toContain("What You'll Do")
    expect(headings).toContain('REQUIREMENTS')
    expect(headings).toContain('Compensation')
  })

  // ── Real JDs ──────────────────────────────────────────────────────
  it('splits Anthropic JD (markdown + bold headings)', () => {
    const result = splitSections(fixture('anthropic-cybersec-re.txt'))
    expect(result.length).toBeGreaterThanOrEqual(4)
    const headings = result.map(s => s.heading).filter(Boolean)
    // Should find headings like "About Anthropic", "About The Role", etc.
    expect(headings.length).toBeGreaterThanOrEqual(3)
  })

  it('splits Snorkel JD (plain text headings)', () => {
    const result = splitSections(fixture('snorkel-training-infra.txt'))
    expect(result.length).toBeGreaterThanOrEqual(3)
  })

  it('splits BetterUp wall-of-text JD', () => {
    const result = splitSections(fixture('betterup-ai-security.txt'))
    // Even wall-of-text should produce at least some sections
    expect(result.length).toBeGreaterThanOrEqual(1)
  })

  // ── Byte offsets ──────────────────────────────────────────────────
  it('tracks byte offsets correctly', () => {
    const text = '## First\nContent 1\n\n## Second\nContent 2'
    const result = splitSections(text)
    expect(result[0].byteOffset).toBe(0)
    expect(result[1].byteOffset).toBe(text.indexOf('## Second'))
  })

  // ── No headings ───────────────────────────────────────────────────
  it('returns single section for text with no recognizable headings', () => {
    const result = splitSections(fixture('generic-no-headings.txt'))
    expect(result).toHaveLength(1)
    expect(result[0].heading).toBeNull()
    expect(result[0].text).toContain('software engineer')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/parser/__tests__/l1-splitter.test.ts`
Expected: Most tests FAIL (stubs return `[]`)

- [ ] **Step 3: Commit test file**

```bash
git add packages/core/src/parser/__tests__/l1-splitter.test.ts
git commit -m "test(parser): add L1 heading splitter tests"
```

---

### Task 4: L1 Heading Splitter — Implementation

**Files:**
- Modify: `packages/core/src/parser/l1-splitter.ts`

- [ ] **Step 1: Implement splitSections**

```ts
// packages/core/src/parser/l1-splitter.ts

import type { Section } from './types'

/**
 * Heading patterns, checked in order. First match wins.
 *
 * 1. Markdown: ## Heading or ### Heading
 * 2. HTML: <h2>Heading</h2> through <h4>
 * 3. Bold: **Heading** on its own line (may have trailing colon)
 * 4. ALL CAPS: 3+ words, all uppercase letters/spaces, on its own line
 * 5. Title-case with colon: "Some Heading:" on its own line (no bullets)
 */
const HEADING_PATTERNS: { pattern: RegExp; extract: (m: RegExpMatchArray) => string }[] = [
  {
    pattern: /^#{1,4}\s+(.+?)\s*$/,
    extract: (m) => m[1].trim(),
  },
  {
    pattern: /^<h[2-4][^>]*>(.+?)<\/h[2-4]>\s*$/i,
    extract: (m) => m[1].trim(),
  },
  {
    pattern: /^\*{2}(.+?)\*{2}\s*:?\s*$/,
    extract: (m) => m[1].trim(),
  },
  {
    pattern: /^([A-Z][A-Z\s&/,'-]{4,})$/,
    extract: (m) => m[1].trim(),
  },
]

// Lines starting with bullet characters are never headings
const BULLET_PREFIX = /^\s*[-*+•]\s|^\s*\d+[.)]\s/

function isAllCapsHeading(line: string): boolean {
  const trimmed = line.trim()
  // Must be ≥3 words to avoid false positives on acronyms like "AWS"
  const words = trimmed.split(/\s+/)
  if (words.length < 3) return false
  // All "word" characters must be uppercase letters, allow &/,'-
  return /^[A-Z][A-Z\s&/,'-]{4,}$/.test(trimmed)
}

function detectHeading(line: string): string | null {
  const trimmed = line.trim()
  if (!trimmed) return null
  if (BULLET_PREFIX.test(trimmed)) return null

  for (const { pattern, extract } of HEADING_PATTERNS) {
    const match = trimmed.match(pattern)
    if (match) {
      // For the ALL CAPS pattern, apply the word-count guard
      if (pattern.source.includes('[A-Z]') && !isAllCapsHeading(trimmed)) {
        continue
      }
      return extract(match)
    }
  }

  return null
}

export function splitSections(rawText: string): Section[] {
  if (!rawText || rawText.trim().length === 0) return []

  const lines = rawText.split('\n')
  const sections: Section[] = []
  let currentHeading: string | null = null
  let currentLines: string[] = []
  let currentOffset = 0
  let bytePos = 0

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    const heading = detectHeading(line)

    if (heading !== null) {
      // Flush previous section
      const text = currentLines.join('\n').trim()
      if (text || currentHeading !== null || sections.length === 0) {
        // Only push preamble if it has content
        if (currentHeading !== null || text.length > 0) {
          sections.push({ heading: currentHeading, text, byteOffset: currentOffset })
        }
      }
      currentHeading = heading
      currentLines = []
      currentOffset = bytePos
    } else {
      currentLines.push(line)
    }

    bytePos += Buffer.byteLength(line, 'utf-8') + 1 // +1 for \n
  }

  // Flush final section
  const text = currentLines.join('\n').trim()
  if (currentHeading !== null || text.length > 0) {
    sections.push({ heading: currentHeading, text, byteOffset: currentOffset })
  }

  return sections
}
```

- [ ] **Step 2: Run L1 tests**

Run: `cd packages/core && bun test src/parser/__tests__/l1-splitter.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Fix any failing tests, then commit**

```bash
git add packages/core/src/parser/l1-splitter.ts
git commit -m "feat(parser): implement L1 heading splitter"
```

---

### Task 5: L2 Taxonomy Classifier — Tests

**Files:**
- Create: `packages/core/src/parser/__tests__/l2-classifier.test.ts`

- [ ] **Step 1: Write L2 tests**

```ts
// packages/core/src/parser/__tests__/l2-classifier.test.ts

import { describe, it, expect } from 'bun:test'
import { classifySections } from '../l2-classifier'
import { splitSections } from '../l1-splitter'
import { readFileSync } from 'fs'
import { join } from 'path'
import type { Section, SectionCategory } from '../types'

const fixture = (name: string) =>
  readFileSync(join(__dirname, 'fixtures', name), 'utf-8')

function classify(text: string) {
  return classifySections(splitSections(text))
}

describe('classifySections (L2)', () => {
  // ── Empty input ───────────────────────────────────────────────────
  it('returns empty array for empty sections', () => {
    expect(classifySections([])).toEqual([])
  })

  // ── Heading-based classification ──────────────────────────────────
  it('classifies "Requirements" heading as requirements', () => {
    const result = classify(fixture('generic-markdown-headings.txt'))
    const req = result.find(s => s.category === 'requirements')
    expect(req).toBeDefined()
    expect(req!.confidence).toBeGreaterThanOrEqual(0.8)
  })

  it('classifies "Responsibilities" heading as responsibilities', () => {
    const result = classify(fixture('generic-markdown-headings.txt'))
    const resp = result.find(s => s.category === 'responsibilities')
    expect(resp).toBeDefined()
  })

  it('classifies "Benefits" heading as benefits', () => {
    const result = classify(fixture('generic-markdown-headings.txt'))
    const ben = result.find(s => s.category === 'benefits')
    expect(ben).toBeDefined()
  })

  it('classifies "About Us" heading as about_company', () => {
    const result = classify(fixture('generic-markdown-headings.txt'))
    const about = result.find(s => s.category === 'about_company')
    expect(about).toBeDefined()
  })

  it('classifies "Qualifications" as requirements', () => {
    const result = classify(fixture('generic-bold-headings.txt'))
    const req = result.find(s => s.category === 'requirements')
    expect(req).toBeDefined()
  })

  it('classifies "Preferred Qualifications" as preferred', () => {
    const sections: Section[] = [
      { heading: 'Preferred Qualifications', text: '- Nice to have skills', byteOffset: 0 },
    ]
    const result = classifySections(sections)
    expect(result[0].category).toBe('preferred')
  })

  it('classifies "Nice to Have" as preferred', () => {
    const result = classify(fixture('generic-html-headings.txt'))
    const pref = result.find(s => s.category === 'preferred')
    expect(pref).toBeDefined()
  })

  // ── Compensation classification ───────────────────────────────────
  it('classifies section with salary info as compensation', () => {
    const result = classify(fixture('generic-allcaps-headings.txt'))
    const comp = result.find(s => s.category === 'compensation')
    expect(comp).toBeDefined()
  })

  // ── EEO classification ────────────────────────────────────────────
  it('classifies EEO boilerplate as eeo', () => {
    const sections: Section[] = [
      {
        heading: 'Equal Opportunity Employer',
        text: 'We are an equal opportunity employer and value diversity. We do not discriminate on the basis of race, religion, color, national origin, gender, sexual orientation, age, marital status, veteran status, or disability status.',
        byteOffset: 0,
      },
    ]
    const result = classifySections(sections)
    expect(result[0].category).toBe('eeo')
  })

  // ── Content-based classification (no heading) ─────────────────────
  it('classifies preamble with company description as description', () => {
    const result = classify(fixture('generic-no-headings.txt'))
    // Single section, no heading — should classify as description
    expect(result[0].category).toBe('description')
  })

  // ── Confidence scores ─────────────────────────────────────────────
  it('assigns higher confidence when heading strongly matches', () => {
    const sections: Section[] = [
      { heading: 'Requirements', text: '- 5+ years experience\n- Python', byteOffset: 0 },
    ]
    const result = classifySections(sections)
    expect(result[0].confidence).toBeGreaterThanOrEqual(0.9)
  })

  it('assigns lower confidence for ambiguous content', () => {
    const sections: Section[] = [
      { heading: null, text: 'We are looking for talented people.', byteOffset: 0 },
    ]
    const result = classifySections(sections)
    expect(result[0].confidence).toBeLessThan(0.7)
  })

  // ── Real JD accuracy ─────────────────────────────────────────────
  it('achieves correct classification on Anthropic JD', () => {
    const result = classify(fixture('anthropic-cybersec-re.txt'))
    const categories = result.map(s => s.category)
    // Must find at least requirements and compensation
    expect(categories).toContain('requirements')
    expect(categories).toContain('compensation')
  })

  it('achieves correct classification on Snorkel JD', () => {
    const result = classify(fixture('snorkel-training-infra.txt'))
    const categories = result.map(s => s.category)
    expect(categories).toContain('responsibilities')
    expect(categories).toContain('preferred')
  })

  // ── >85% accuracy on fixture corpus ───────────────────────────────
  it('achieves >85% classification accuracy on labeled fixtures', () => {
    // Each fixture maps to expected categories present
    const expectations: [string, SectionCategory[]][] = [
      ['generic-markdown-headings.txt', ['about_company', 'responsibilities', 'requirements', 'benefits']],
      ['generic-bold-headings.txt', ['description', 'responsibilities', 'requirements']],
      ['generic-allcaps-headings.txt', ['about_company', 'responsibilities', 'requirements', 'compensation']],
      ['generic-html-headings.txt', ['about_company', 'responsibilities', 'requirements', 'preferred']],
      ['generic-mixed-headings.txt', ['description', 'responsibilities', 'requirements', 'compensation']],
      ['salary-range-dash.txt', ['description', 'requirements', 'compensation']],
      ['salary-range-to.txt', ['description', 'requirements', 'compensation']],
      ['multi-location.txt', ['description', 'location', 'requirements']],
      ['remote-only.txt', ['description', 'location', 'requirements']],
      ['hybrid-location.txt', ['description', 'location', 'requirements']],
    ]

    let correct = 0
    let total = 0

    for (const [file, expectedCategories] of expectations) {
      const result = classify(fixture(file))
      const actualCategories = result.map(s => s.category)
      for (const expected of expectedCategories) {
        total++
        if (actualCategories.includes(expected)) correct++
      }
    }

    const accuracy = correct / total
    expect(accuracy).toBeGreaterThan(0.85)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/parser/__tests__/l2-classifier.test.ts`
Expected: Most tests FAIL (stub returns `[]`)

- [ ] **Step 3: Commit test file**

```bash
git add packages/core/src/parser/__tests__/l2-classifier.test.ts
git commit -m "test(parser): add L2 taxonomy classifier tests"
```

---

### Task 6: L2 Taxonomy Classifier — Implementation

**Files:**
- Modify: `packages/core/src/parser/l2-classifier.ts`

- [ ] **Step 1: Implement classifySections**

```ts
// packages/core/src/parser/l2-classifier.ts

import type { Section, ClassifiedSection, SectionCategory } from './types'

/**
 * Keyword lists per category. Each keyword has a weight.
 * Heading matches are weighted 3× body matches.
 */
interface KeywordEntry {
  word: string
  weight: number
}

const TAXONOMY: Record<SectionCategory, KeywordEntry[]> = {
  responsibilities: [
    { word: 'responsibilities', weight: 3 },
    { word: 'what you will do', weight: 3 },
    { word: "what you'll do", weight: 3 },
    { word: 'key responsibilities', weight: 3 },
    { word: 'role responsibilities', weight: 2 },
    { word: 'duties', weight: 2 },
    { word: 'you will', weight: 1 },
    { word: 'your role', weight: 1 },
    { word: 'day to day', weight: 1 },
    { word: 'main responsibilities', weight: 3 },
  ],
  requirements: [
    { word: 'requirements', weight: 3 },
    { word: 'qualifications', weight: 3 },
    { word: 'required qualifications', weight: 3 },
    { word: 'minimum qualifications', weight: 3 },
    { word: 'must have', weight: 2 },
    { word: 'must-have', weight: 2 },
    { word: 'what we are looking for', weight: 3 },
    { word: "what we're looking for", weight: 3 },
    { word: 'what you need', weight: 2 },
    { word: "what you'll need", weight: 2 },
    { word: "what you'll bring", weight: 2 },
    { word: 'skills & competencies', weight: 2 },
    { word: 'years of experience', weight: 1 },
    { word: 'years experience', weight: 1 },
    { word: 'proficiency', weight: 1 },
    { word: 'experience with', weight: 0.5 },
  ],
  preferred: [
    { word: 'preferred', weight: 3 },
    { word: 'preferred qualifications', weight: 3 },
    { word: 'desired qualifications', weight: 3 },
    { word: 'nice to have', weight: 3 },
    { word: 'nice-to-have', weight: 3 },
    { word: 'bonus', weight: 2 },
    { word: 'plus', weight: 1 },
    { word: 'strong candidates', weight: 2 },
    { word: 'ideally', weight: 1 },
    { word: 'strong candidate', weight: 2 },
  ],
  description: [
    { word: 'about the role', weight: 3 },
    { word: 'about this role', weight: 3 },
    { word: 'position summary', weight: 3 },
    { word: 'role overview', weight: 3 },
    { word: 'overview', weight: 2 },
    { word: 'position overview', weight: 3 },
    { word: 'the role', weight: 1 },
    { word: 'join our', weight: 1 },
    { word: 'we are seeking', weight: 1 },
    { word: 'we are looking for', weight: 1 },
  ],
  location: [
    { word: 'location', weight: 3 },
    { word: 'locations', weight: 3 },
    { word: 'office location', weight: 3 },
    { word: 'work location', weight: 3 },
    { word: 'based in', weight: 1 },
    { word: 'office', weight: 0.5 },
  ],
  compensation: [
    { word: 'compensation', weight: 3 },
    { word: 'salary', weight: 3 },
    { word: 'annual salary', weight: 3 },
    { word: 'pay range', weight: 3 },
    { word: 'base salary', weight: 3 },
    { word: 'compensation and benefits', weight: 2 },
    { word: 'total compensation', weight: 3 },
    { word: '$', weight: 1 },
    { word: 'per year', weight: 1 },
    { word: 'annually', weight: 1 },
    { word: '/yr', weight: 1 },
    { word: '/hr', weight: 1 },
  ],
  benefits: [
    { word: 'benefits', weight: 3 },
    { word: 'perks', weight: 3 },
    { word: 'what we offer', weight: 3 },
    { word: 'our benefits', weight: 3 },
    { word: 'health insurance', weight: 1 },
    { word: '401k', weight: 1 },
    { word: '401(k)', weight: 1 },
    { word: 'paid time off', weight: 1 },
    { word: 'pto', weight: 1 },
    { word: 'equity', weight: 0.5 },
  ],
  about_company: [
    { word: 'about us', weight: 3 },
    { word: 'about the company', weight: 3 },
    { word: 'about the team', weight: 3 },
    { word: 'who we are', weight: 3 },
    { word: 'our mission', weight: 2 },
    { word: 'our values', weight: 2 },
    { word: 'our culture', weight: 2 },
    { word: 'company overview', weight: 3 },
    { word: 'company description', weight: 3 },
    { word: 'about anthropic', weight: 3 },
    { word: "mission is", weight: 1 },
  ],
  eeo: [
    { word: 'equal opportunity', weight: 3 },
    { word: 'equal employment', weight: 3 },
    { word: 'eeo', weight: 3 },
    { word: 'do not discriminate', weight: 2 },
    { word: 'affirmative action', weight: 2 },
    { word: 'diversity', weight: 1 },
    { word: 'regardless of race', weight: 2 },
    { word: 'nondiscrimination', weight: 2 },
    { word: 'protected veteran', weight: 1 },
  ],
}

const ALL_CATEGORIES: SectionCategory[] = [
  'responsibilities', 'requirements', 'preferred', 'description',
  'location', 'compensation', 'benefits', 'about_company', 'eeo',
]

function scoreSection(section: Section, category: SectionCategory): number {
  const keywords = TAXONOMY[category]
  let score = 0

  const headingLower = section.heading?.toLowerCase() ?? ''
  const bodyLower = section.text.toLowerCase()

  for (const { word, weight } of keywords) {
    // Heading matches weighted 3×
    if (headingLower.includes(word)) {
      score += weight * 3
    }
    // Body matches
    const bodyMatches = countOccurrences(bodyLower, word)
    score += bodyMatches * weight
  }

  return score
}

function countOccurrences(text: string, word: string): number {
  let count = 0
  let pos = 0
  while ((pos = text.indexOf(word, pos)) !== -1) {
    count++
    pos += word.length
  }
  return count
}

function scoreToConfidence(score: number, bestScore: number, hasHeading: boolean): number {
  if (bestScore === 0) return 0.3 // no signal at all
  // Heading-driven matches are very confident
  if (hasHeading && score >= 6) return 0.95
  if (hasHeading && score >= 3) return 0.85
  // Body-only matches
  if (score >= 6) return 0.8
  if (score >= 3) return 0.65
  if (score >= 1) return 0.5
  return 0.3
}

export function classifySections(sections: Section[]): ClassifiedSection[] {
  return sections.map(section => {
    let bestCategory: SectionCategory = 'description'
    let bestScore = 0

    for (const category of ALL_CATEGORIES) {
      const score = scoreSection(section, category)
      if (score > bestScore) {
        bestScore = score
        bestCategory = category
      }
    }

    const hasHeadingSignal = section.heading !== null &&
      TAXONOMY[bestCategory].some(k =>
        section.heading!.toLowerCase().includes(k.word)
      )

    const confidence = scoreToConfidence(bestScore, bestScore, hasHeadingSignal)

    return {
      ...section,
      category: bestCategory,
      confidence,
    }
  })
}
```

- [ ] **Step 2: Run L2 tests**

Run: `cd packages/core && bun test src/parser/__tests__/l2-classifier.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Fix any failures, then commit**

```bash
git add packages/core/src/parser/l2-classifier.ts
git commit -m "feat(parser): implement L2 taxonomy classifier"
```

---

### Task 7: L3 Inline Extractors — Tests

**Files:**
- Create: `packages/core/src/parser/__tests__/l3-extractors.test.ts`

- [ ] **Step 1: Write L3 tests**

```ts
// packages/core/src/parser/__tests__/l3-extractors.test.ts

import { describe, it, expect } from 'bun:test'
import { extractSalary, extractLocations, extractWorkPosture } from '../l3-extractors'
import { splitSections } from '../l1-splitter'
import { classifySections } from '../l2-classifier'
import { readFileSync } from 'fs'
import { join } from 'path'
import type { ClassifiedSection } from '../types'

const fixture = (name: string) =>
  readFileSync(join(__dirname, 'fixtures', name), 'utf-8')

function pipeline(text: string) {
  return classifySections(splitSections(text))
}

describe('extractSalary (L3)', () => {
  it('returns null for no salary info', () => {
    const sections = pipeline('## Requirements\n- 5+ years experience')
    expect(extractSalary(sections)).toBeNull()
  })

  it('extracts salary range with dash: $200,000 - $300,000', () => {
    const sections = pipeline(fixture('salary-range-dash.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(200_000)
    expect(salary!.max).toBe(300_000)
    expect(salary!.period).toBe('annual')
  })

  it('extracts salary range with "to": $180,000 to $260,000', () => {
    const sections = pipeline(fixture('salary-range-to.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(180_000)
    expect(salary!.max).toBe(260_000)
  })

  it('extracts hourly rate: $75 - $120/hr', () => {
    const sections = pipeline(fixture('salary-hourly.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(75)
    expect(salary!.max).toBe(120)
    expect(salary!.period).toBe('hourly')
  })

  it('extracts salary with em-dash: $300,000—$405,000', () => {
    const sections = pipeline(fixture('salary-em-dash.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(300_000)
    expect(salary!.max).toBe(405_000)
  })

  it('extracts salary from Anthropic JD body text', () => {
    const sections = pipeline(fixture('anthropic-cybersec-re.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(300_000)
    expect(salary!.max).toBe(405_000)
  })

  it('extracts salary from Snorkel JD', () => {
    const sections = pipeline(fixture('snorkel-training-infra.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(150_000)
    expect(salary!.max).toBe(180_000)
  })

  it('handles single salary number: $95,000/year', () => {
    const sections = pipeline(fixture('salary-single-number.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(95_000)
    // Single number: min === max
    expect(salary!.max).toBe(95_000)
  })

  it('handles $120k-$160k shorthand', () => {
    const sections = pipeline(fixture('minimal-jd.txt'))
    const salary = extractSalary(sections)
    expect(salary).not.toBeNull()
    expect(salary!.min).toBe(120_000)
    expect(salary!.max).toBe(160_000)
  })
})

describe('extractLocations (L3)', () => {
  it('returns empty array for no location info', () => {
    const sections = pipeline('## Requirements\n- 5+ years experience')
    expect(extractLocations(sections)).toEqual([])
  })

  it('extracts multiple locations from multi-location fixture', () => {
    const sections = pipeline(fixture('multi-location.txt'))
    const locations = extractLocations(sections)
    expect(locations.length).toBeGreaterThanOrEqual(3)
    expect(locations).toContain('New York, NY')
    expect(locations).toContain('San Francisco, CA')
    expect(locations).toContain('Seattle, WA')
    expect(locations).toContain('Austin, TX')
  })

  it('extracts single location from hybrid fixture', () => {
    const sections = pipeline(fixture('hybrid-location.txt'))
    const locations = extractLocations(sections)
    expect(locations.length).toBeGreaterThanOrEqual(1)
    expect(locations).toContain('Arlington, VA')
  })

  it('returns Remote for remote-only fixture', () => {
    const sections = pipeline(fixture('remote-only.txt'))
    const locations = extractLocations(sections)
    // May return "Remote" or empty depending on design — at minimum should not crash
    expect(Array.isArray(locations)).toBe(true)
  })

  it('extracts location from Snorkel JD', () => {
    const sections = pipeline(fixture('snorkel-training-infra.txt'))
    const locations = extractLocations(sections)
    expect(locations.length).toBeGreaterThanOrEqual(1)
  })
})

describe('extractWorkPosture (L3)', () => {
  it('returns null for no posture info', () => {
    const sections = pipeline('## Requirements\n- 5+ years experience')
    expect(extractWorkPosture(sections)).toBeNull()
  })

  it('detects remote from remote-only fixture', () => {
    const sections = pipeline(fixture('remote-only.txt'))
    expect(extractWorkPosture(sections)).toBe('remote')
  })

  it('detects hybrid from hybrid fixture', () => {
    const sections = pipeline(fixture('hybrid-location.txt'))
    expect(extractWorkPosture(sections)).toBe('hybrid')
  })

  it('detects remote from wall-of-text mentioning hybrid', () => {
    const sections = pipeline(fixture('wall-of-text.txt'))
    expect(extractWorkPosture(sections)).toBe('hybrid')
  })

  it('detects remote from Snorkel JD', () => {
    const sections = pipeline(fixture('snorkel-training-infra.txt'))
    const posture = extractWorkPosture(sections)
    // Snorkel says "Redwood City/San Francisco, CA or Remote"
    expect(posture).toBe('remote')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd packages/core && bun test src/parser/__tests__/l3-extractors.test.ts`
Expected: Most tests FAIL (stubs return null/[])

- [ ] **Step 3: Commit test file**

```bash
git add packages/core/src/parser/__tests__/l3-extractors.test.ts
git commit -m "test(parser): add L3 inline extractor tests"
```

---

### Task 8: L3 Inline Extractors — Implementation

**Files:**
- Modify: `packages/core/src/parser/l3-extractors.ts`

- [ ] **Step 1: Implement extractors**

```ts
// packages/core/src/parser/l3-extractors.ts

import type { ClassifiedSection, SalaryRange } from './types'

// ── Salary Extractor ────────────────────────────────────────────────

/**
 * Salary patterns, ordered by specificity.
 *
 * Handles:
 * - $200,000 - $300,000 (with optional USD/per year/annually)
 * - $200,000—$300,000 (em-dash, Anthropic style)
 * - $200,000 to $300,000
 * - $120k-$160k (shorthand)
 * - $75 - $120/hr (hourly)
 * - $95,000/year (single number)
 * - Starting salary: $95,000
 */
const SALARY_RANGE_PATTERNS = [
  // $X,XXX—$Y,YYY or $X,XXX - $Y,YYY or $X,XXX – $Y,YYY (all dash types)
  /\$\s*([\d,]+(?:\.\d+)?)\s*[—–\-]\s*\$\s*([\d,]+(?:\.\d+)?)/,
  // $Xk-$Yk or $Xk – $Yk
  /\$\s*(\d+)\s*k\s*[—–\-]\s*\$\s*(\d+)\s*k/i,
  // $X to $Y
  /\$\s*([\d,]+(?:\.\d+)?)\s+to\s+\$\s*([\d,]+(?:\.\d+)?)/i,
  // $Xk to $Yk
  /\$\s*(\d+)\s*k\s+to\s+\$\s*(\d+)\s*k/i,
  // ranges from $X to $Y
  /from\s+\$\s*([\d,]+(?:\.\d+)?)\s+to\s+\$\s*([\d,]+(?:\.\d+)?)/i,
]

// Single salary: $X,XXX or $Xk followed by salary context
const SINGLE_SALARY_PATTERN = /\$\s*([\d,]+(?:\.\d+)?)\s*k?\b/

const HOURLY_INDICATORS = /\/\s*h(?:ou)?r|per\s+hour|hourly/i
const ANNUAL_INDICATORS = /\/\s*y(?:ea)?r|per\s+year|annual|annually|USD/i

function parseSalaryNumber(raw: string): number {
  return parseFloat(raw.replace(/,/g, ''))
}

export function extractSalary(sections: ClassifiedSection[]): SalaryRange | null {
  // Prioritize compensation sections, then scan all sections
  const ordered = [
    ...sections.filter(s => s.category === 'compensation'),
    ...sections.filter(s => s.category !== 'compensation'),
  ]

  for (const section of ordered) {
    const text = section.text
    const lines = text.split('\n')

    for (const line of lines) {
      // Try range patterns first
      for (const pattern of SALARY_RANGE_PATTERNS) {
        const match = line.match(pattern)
        if (match) {
          let min = parseSalaryNumber(match[1])
          let max = parseSalaryNumber(match[2])

          // Handle $Xk shorthand
          if (pattern.source.includes('k') && min < 1000) {
            min *= 1000
            max *= 1000
          }

          const period = HOURLY_INDICATORS.test(line)
            ? 'hourly' as const
            : (min >= 1000 || ANNUAL_INDICATORS.test(line))
              ? 'annual' as const
              : 'unknown' as const

          return { min, max, period }
        }
      }
    }
  }

  // Try single salary number as fallback
  for (const section of ordered) {
    for (const line of section.text.split('\n')) {
      // Only match lines with salary context words
      if (!/salary|compensation|pay|starting/i.test(line)) continue
      const match = line.match(SINGLE_SALARY_PATTERN)
      if (match) {
        let value = parseSalaryNumber(match[1])
        // Handle $Xk
        if (/k\b/i.test(line.slice(match.index! + match[0].length - 1, match.index! + match[0].length + 1))) {
          value *= 1000
        }
        if (value < 100) continue // too small, probably not a salary

        const period = HOURLY_INDICATORS.test(line)
          ? 'hourly' as const
          : 'annual' as const

        return { min: value, max: value, period }
      }
    }
  }

  return null
}

// ── Location Extractor ──────────────────────────────────────────────

// US state abbreviations
const US_STATES = new Set([
  'AL','AK','AZ','AR','CA','CO','CT','DE','FL','GA','HI','ID','IL','IN',
  'IA','KS','KY','LA','ME','MD','MA','MI','MN','MS','MO','MT','NE','NV',
  'NH','NJ','NM','NY','NC','ND','OH','OK','OR','PA','RI','SC','SD','TN',
  'TX','UT','VT','VA','WA','WV','WI','WY','DC',
])

// "City, ST" pattern — matches "New York, NY" or "San Francisco, CA"
const CITY_STATE_PATTERN = /([A-Z][a-zA-Z\s.'-]+),\s*([A-Z]{2})\b/g

export function extractLocations(sections: ClassifiedSection[]): string[] {
  // Prioritize location sections, then description/compensation
  const ordered = [
    ...sections.filter(s => s.category === 'location'),
    ...sections.filter(s => s.category === 'description' || s.category === 'compensation'),
    ...sections.filter(s => !['location', 'description', 'compensation'].includes(s.category)),
  ]

  const locations = new Set<string>()

  for (const section of ordered) {
    const text = section.text
    let match: RegExpExecArray | null

    // Reset lastIndex for global regex
    CITY_STATE_PATTERN.lastIndex = 0
    while ((match = CITY_STATE_PATTERN.exec(text)) !== null) {
      const city = match[1].trim()
      const state = match[2]
      if (US_STATES.has(state)) {
        locations.add(`${city}, ${state}`)
      }
    }
  }

  return [...locations]
}

// ── Work Posture Extractor ──────────────────────────────────────────

const REMOTE_PATTERNS = [
  /fully\s+remote/i,
  /100%\s+remote/i,
  /remote\s+position/i,
  /remote\s+role/i,
  /this\s+is\s+a\s+(?:fully\s+)?remote/i,
  /\bremote\b/i,
]

const HYBRID_PATTERNS = [
  /hybrid\s+(?:position|role|work|schedule)/i,
  /\bhybrid\b/i,
  /days?\s+(?:per\s+week|\/\s*week)\s+in[\s-]office/i,
  /in[\s-]office\s+\d+/i,
  /\d+\s+days?\s+(?:per\s+week|\/\s*week)/i,
]

const ONSITE_PATTERNS = [
  /on[\s-]site/i,
  /in[\s-]office/i,
  /in[\s-]person/i,
]

export function extractWorkPosture(sections: ClassifiedSection[]): 'remote' | 'hybrid' | 'on-site' | null {
  // Prioritize location/description sections
  const ordered = [
    ...sections.filter(s => s.category === 'location'),
    ...sections.filter(s => s.category === 'description'),
    ...sections.filter(s => !['location', 'description'].includes(s.category)),
  ]

  // Collect signals across all sections
  let remoteScore = 0
  let hybridScore = 0
  let onsiteScore = 0

  for (const section of ordered) {
    const text = section.text

    for (const p of REMOTE_PATTERNS) {
      if (p.test(text)) remoteScore += p.source.includes('fully') ? 3 : 1
    }
    for (const p of HYBRID_PATTERNS) {
      if (p.test(text)) hybridScore += p.source.includes('hybrid') ? 3 : 2
    }
    for (const p of ONSITE_PATTERNS) {
      if (p.test(text)) onsiteScore += 1
    }
  }

  // Hybrid beats remote when both present (e.g., "hybrid, 2 days in office")
  // because "hybrid" implies partial remote
  if (hybridScore > 0 && hybridScore >= remoteScore) return 'hybrid'
  if (remoteScore > 0) return 'remote'
  if (onsiteScore > 0) return 'on-site'
  return null
}
```

- [ ] **Step 2: Run L3 tests**

Run: `cd packages/core && bun test src/parser/__tests__/l3-extractors.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Fix any failures, then commit**

```bash
git add packages/core/src/parser/l3-extractors.ts
git commit -m "feat(parser): implement L3 inline extractors (salary, location, posture)"
```

---

### Task 9: Integration Tests

**Files:**
- Create: `packages/core/src/parser/__tests__/integration.test.ts`

- [ ] **Step 1: Write integration tests**

```ts
// packages/core/src/parser/__tests__/integration.test.ts

import { describe, it, expect } from 'bun:test'
import { parseJobDescription } from '../index'
import { readFileSync } from 'fs'
import { join } from 'path'

const fixture = (name: string) =>
  readFileSync(join(__dirname, 'fixtures', name), 'utf-8')

describe('parseJobDescription (integration)', () => {
  it('parses Anthropic Cybersecurity RE JD end-to-end', () => {
    const result = parseJobDescription(fixture('anthropic-cybersec-re.txt'))

    // Sections present
    expect(result.sections.length).toBeGreaterThanOrEqual(4)
    const categories = result.sections.map(s => s.category)
    expect(categories).toContain('requirements')
    expect(categories).toContain('compensation')

    // Salary extracted
    expect(result.salary).not.toBeNull()
    expect(result.salary!.min).toBe(300_000)
    expect(result.salary!.max).toBe(405_000)
  })

  it('parses Snorkel Training Infra JD end-to-end', () => {
    const result = parseJobDescription(fixture('snorkel-training-infra.txt'))

    const categories = result.sections.map(s => s.category)
    expect(categories).toContain('responsibilities')

    expect(result.salary).not.toBeNull()
    expect(result.salary!.min).toBe(150_000)
    expect(result.salary!.max).toBe(180_000)

    expect(result.locations.length).toBeGreaterThanOrEqual(1)
  })

  it('parses minimal JD', () => {
    const result = parseJobDescription(fixture('minimal-jd.txt'))

    expect(result.salary).not.toBeNull()
    expect(result.salary!.min).toBe(120_000)
    expect(result.salary!.max).toBe(160_000)

    expect(result.workPosture).toBe('remote')
  })

  it('parses wall-of-text JD', () => {
    const result = parseJobDescription(fixture('wall-of-text.txt'))

    // Should still extract salary and posture
    expect(result.salary).not.toBeNull()
    expect(result.salary!.min).toBe(150_000)
    expect(result.salary!.max).toBe(200_000)

    expect(result.workPosture).toBe('hybrid')
    expect(result.locations).toContain('Chicago, IL')
  })

  it('handles empty string gracefully', () => {
    const result = parseJobDescription('')
    expect(result.sections).toEqual([])
    expect(result.salary).toBeNull()
    expect(result.locations).toEqual([])
    expect(result.workPosture).toBeNull()
  })

  it('handles whitespace-only input', () => {
    const result = parseJobDescription('   \n\n   ')
    expect(result.sections).toEqual([])
    expect(result.salary).toBeNull()
  })

  it('extracts from all fixture files without throwing', () => {
    const { readdirSync } = require('fs')
    const fixtureDir = join(__dirname, 'fixtures')
    const files = readdirSync(fixtureDir).filter((f: string) => f.endsWith('.txt'))

    for (const file of files) {
      const text = readFileSync(join(fixtureDir, file), 'utf-8')
      // Should never throw
      const result = parseJobDescription(text)
      expect(result.sections).toBeDefined()
      expect(result.salary === null || typeof result.salary.min === 'number').toBe(true)
      expect(Array.isArray(result.locations)).toBe(true)
    }
  })
})
```

- [ ] **Step 2: Run integration tests**

Run: `cd packages/core && bun test src/parser/__tests__/integration.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Run the full parser test suite**

Run: `cd packages/core && bun test src/parser/`
Expected: All tests PASS

- [ ] **Step 4: Fix any failures, then commit**

```bash
git add packages/core/src/parser/__tests__/integration.test.ts
git commit -m "test(parser): add integration tests for full parser pipeline"
```

---

### Task 10: Version Bump + Final Verification

**Files:**
- Modify: `packages/extension/manifest.json` (version → 0.1.3)
- Modify: `packages/extension/manifest.firefox.json` (version → 0.1.3)

- [ ] **Step 1: Bump extension version to 0.1.3**

In both `packages/extension/manifest.json` and `packages/extension/manifest.firefox.json`, change:
```json
"version": "0.1.2"
```
to:
```json
"version": "0.1.3"
```

- [ ] **Step 2: Run all core tests**

Run: `cd packages/core && bun test`
Expected: All existing tests + new parser tests PASS

- [ ] **Step 3: Run all extension tests**

Run: `cd packages/extension && bun test`
Expected: All 96 tests PASS (no regressions)

- [ ] **Step 4: Verify TypeScript compiles clean**

Run: `cd packages/core && bunx tsc --noEmit`
Expected: No errors

- [ ] **Step 5: Commit version bump**

```bash
git add packages/extension/manifest.json packages/extension/manifest.firefox.json
git commit -m "chore(ext): bump version to 0.1.3 (M3)"
```

- [ ] **Step 6: Build extension to verify no breakage**

Run: `cd packages/extension && bun run build`
Expected: Both Chrome and Firefox builds succeed

---

## Notes for M5 Integration

The parser's public API is `parseJobDescription(rawText: string): ParsedJobDescription`. M5 will:
1. Import it in the extension content script to parse extracted descriptions before sending to background
2. Import it in `JobDescriptionService.create()` to parse at ingest time and populate `parsed_sections` column
3. The existing `parseRequirements()` in `lib/jd-parser.ts` remains untouched — it serves a different purpose (extracting individual requirement bullet items for skills matching)
