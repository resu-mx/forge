# Workflow

## Views

- (MVP) **Sources list/editor** — create and edit source descriptions
- (MVP) **Derivation view** — select a source, trigger bullet generation, review/approve/reject inline
- (MVP) **Perspective view** — select a bullet, trigger perspective generation, review/approve/reject inline
- (MVP) **Chain view** — visualize Source→Bullet→Perspective provenance for any entity
- (MVP) **Resume builder** — drag/arrange approved perspectives, see gap analysis, trigger export

- Application Tracking
  - Status
  - Prioritized Jobs
  - GANTT
- TODOs
  - GANTT
  - Global
- Calendar
- Timeline
- Resume
  - Header (name, contact)
  - summary
  - work history (grouped by org/role with bullets underneath)
  - education
  - skills
  - certifications
- CV
- Bullets

Yes — right now the resume builder assembles entries but there's no preview of what the resume actually looks like. 
You need a render view that shows the resume as a formatted document (the way a recruiter would see it). This could be:

- A "Preview" tab/panel in the resume builder that renders entries in resume format: header (name, contact), summary, work history (grouped by org/role with bullets underneath), education, skills, certifications
- Uses the chain data: entry → perspective → bullet → source (role) to group bullets under the right employer/role
- Clean typography, printable layout (A4/Letter)
- Eventually this is what the "Export" button would render to PDF

This is a new feature — not in the current spec. Worth adding as a Phase 16 task or extending the resume builder view.

## Browser Extensions

- GreenHouse
- LinkedIn
- 

## Modules

### Core Lib

- APIs
  - WebCrawler
  - WebScrapers
  - Browser Automation
- Indexers
- Databases
- Parsers
  - LaTeX
  - Markdown
  - PDF
- Chunkers
- Embedders

### SDK

- NLP descriptions → structured events → perspective bullets) and chain-of-custody tracking
- layer between "source" and "bullet" for structured event extraction (dates, technologies, outcomes, team size, etc.)?
- SQLite schema has bullets with framing (accomplishment/responsibility/context) and junction tables linking them to skills, roles, and resumes.
  - existing schema tracks clearances, education, awards, publications, research
  - work | project | education | certification | clearance | award | publication | research
  - tracks clearances, languages, awards, research, publications — Forge Sources have no path to these
- archetypes
- Perspectives.framing (accomplishment/responsibility/context) vs existing bullets.framing (systems_engineering/security/etc) — different axes, same field name
- WebSpiders
- API Workflows
- DataType Schemas
- Database Workflows
- Job Sites Integrations
  - Indeed: Browser Automation
  - LinkedIn: API + Browser Automation
  - JobSpy: API
  - Monster
  - Greenhouse: API / WebCrawler / Browser Automation

## Data Stores

Data model: Sources → Bullets → Perspectives, all in SQLite, with foreign keys enforcing the chain. Each entity has a status field (draft/approved/etc.) to gate the human review requirement.
Resume generation: AI selects from approved Perspectives, recommends new Perspectives from existing Bullets when gaps exist, or flags to human. The chain is always auditable.
SQLite for discovery: Yes — programmatic queries for coverage analysis, skill matching, gap detection. Keep token costs low by doing the filtering in SQL, only sending relevant bullets to AI.

### Schemas

### Types

- resume
  - summary
  - title
  - experience
    - bullets
    - time-range
    - org
    - role
  - skills
    - category: list[]
  - Education
  - Certifications
  - Security Clearance
  - Projects/Portfolio
  - Presentations
- CoverLetter
  - Why <Org>
  - Why <Role>
  - Why <Role> @ <Org>
- Personal Site(s)
  - blog
  - portfolio
  - github
  - linkedin
  - etc
- organization
- experience
- project
- sprint
- role
- research
- writing
- talking
- cv
- education
- job-post
  - Key Responsibilities
  - Qualifications
    - Required
    - Preferred
  - Compensation
- JD Decomposition
  - requirement
  - level
  - category
- Evidence Mapping
  - Requirement
  - Evidence
  - Location
  - Verdict
- Problem Identification
  - Critical v Important v Minor
- Alignment Plan
  - Problem
  - Fix
    - Reframe
    - Add
    - Remove
    - Highlight Gap (Internal v External)
  - Where

## Scores

- Alignment
  - Target Roles
  - Target Worklife
    - Remote/On-Site/Hybrid, Org Profile
  - Target Compensation
    - Benefits, RSU, Equity, Base, etc
  - Experience

<!--recipes-->
<!--foo-->
<!--guru-->
<!--store-->
<!--storage-->
<!--studio-->
<!--space-->
<!--tools-->
<!--wiki-->
<!--nexus-->
<!--works-->


# Jobs Crate

This crate provides a Rust types, traits, and functions for working with job listings.

Job Search Crates

```asciidoc
┌──────────────┬───────────────────────────────────────────┬───────────┐
│    Crate     │                Description                │ Downloads │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ jobshell     │ CLI for job searching and scraping boards │ 13,216    │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ google-jobs4 │ Google Cloud Talent Solution API (v4)     │ 17,172    │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ google-jobs3 │ Google Cloud Talent Solution API (v3)     │ 27,121    │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ jobsuche     │ German employment agency API              │ 881       │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ fetters      │ CLI for tracking job applications         │ 430       │
└──────────────┴───────────────────────────────────────────┴───────────┘
```

# Indeed Crate

This crate provides a Rust interface to the Indeed API.

  Indeed Crates
```asciidoc
  ┌─────────────────────┬────────────────────────────────────────┬───────────┐
  │        Crate        │              Description               │ Downloads │
  ├─────────────────────┼────────────────────────────────────────┼───────────┤
  │ indeed_scraper      │ Indeed.com job scraper                 │ 1,809     │
  ├─────────────────────┼────────────────────────────────────────┼───────────┤
  │ indeed_querybuilder │ Indeed.com search builder for scraping │ 5,318     │
  └─────────────────────┴────────────────────────────────────────┴───────────┘
```

# LinkedIn Crate

This crate provides a Rust interface to the LinkedIn API.

LinkedIn Crates
```asciidoc
┌───────────────────────┬────────────────────────────────────────┬───────────┐
│         Crate         │              Description               │ Downloads │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ proxycurl-linkedin-rs │ Rust client for Proxycurl LinkedIn API │ 2,829     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ oauth2-linkedin       │ OAuth 2.0 for LinkedIn                 │ 5,077     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ gen-linkedin          │ Minimal LinkedIn Posts API client      │ 356       │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ linkedin-csv          │ Structs for LinkedIn data exports      │ 3,105     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ credify               │ LinkedIn profile URL validator         │ 1,795     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ netprune              │ LinkedIn connection management         │ 33        │
└───────────────────────┴────────────────────────────────────────┴───────────┘
```

```asciidoc
Rust Crates for Job Search APIs

LinkedIn Crates
┌───────────────────────┬────────────────────────────────────────┬───────────┐
│         Crate         │              Description               │ Downloads │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ proxycurl-linkedin-rs │ Rust client for Proxycurl LinkedIn API │ 2,829     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ oauth2-linkedin       │ OAuth 2.0 for LinkedIn                 │ 5,077     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ gen-linkedin          │ Minimal LinkedIn Posts API client      │ 356       │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ linkedin-csv          │ Structs for LinkedIn data exports      │ 3,105     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ credify               │ LinkedIn profile URL validator         │ 1,795     │
├───────────────────────┼────────────────────────────────────────┼───────────┤
│ netprune              │ LinkedIn connection management         │ 33        │
└───────────────────────┴────────────────────────────────────────┴───────────┘

Indeed Crates
┌─────────────────────┬────────────────────────────────────────┬───────────┐
│        Crate        │              Description               │ Downloads │
├─────────────────────┼────────────────────────────────────────┼───────────┤
│ indeed_scraper      │ Indeed.com job scraper                 │ 1,809     │
├─────────────────────┼────────────────────────────────────────┼───────────┤
│ indeed_querybuilder │ Indeed.com search builder for scraping │ 5,318     │
└─────────────────────┴────────────────────────────────────────┴───────────┘

Job Search Crates
┌──────────────┬───────────────────────────────────────────┬───────────┐
│    Crate     │                Description                │ Downloads │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ jobshell     │ CLI for job searching and scraping boards │ 13,216    │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ google-jobs4 │ Google Cloud Talent Solution API (v4)     │ 17,172    │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ google-jobs3 │ Google Cloud Talent Solution API (v3)     │ 27,121    │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ jobsuche     │ German employment agency API              │ 881       │
├──────────────┼───────────────────────────────────────────┼───────────┤
│ fetters      │ CLI for tracking job applications         │ 430       │
└──────────────┴───────────────────────────────────────────┴───────────┘

Best Options
┌──────────────────────────┬──────────────────────────────────────┐
│         Use Case         │          Recommended Crate           │
├──────────────────────────┼──────────────────────────────────────┤
│ LinkedIn OAuth           │ oauth2-linkedin                      │
├──────────────────────────┼──────────────────────────────────────┤
│ LinkedIn Data (paid API) │ proxycurl-linkedin-rs                │
├──────────────────────────┼──────────────────────────────────────┤
│ Indeed Scraping          │ indeed_scraper + indeed_querybuilder │
├──────────────────────────┼──────────────────────────────────────┤
│ Google Jobs API          │ google-jobs4                         │
├──────────────────────────┼──────────────────────────────────────┤
│ General Job CLI          │ jobshell                             │
└──────────────────────────┴──────────────────────────────────────┘
```

# Jobs Crate

This crate provides a Rust types, traits, and functions for defining and managing organizations.


## Best Options

```asciidoc
  ┌──────────────────────────┬──────────────────────────────────────┐
  │         Use Case         │          Recommended Crate           │
  ├──────────────────────────┼──────────────────────────────────────┤
  │ LinkedIn OAuth           │ oauth2-linkedin                      │
  ├──────────────────────────┼──────────────────────────────────────┤
  │ LinkedIn Data (paid API) │ proxycurl-linkedin-rs                │
  ├──────────────────────────┼──────────────────────────────────────┤
  │ Indeed Scraping          │ indeed_scraper + indeed_querybuilder │
  ├──────────────────────────┼──────────────────────────────────────┤
  │ Google Jobs API          │ google-jobs4                         │
  ├──────────────────────────┼──────────────────────────────────────┤
  │ General Job CLI          │ jobshell                             │
  └──────────────────────────┴──────────────────────────────────────┘
```

**Note**: There's no official Indeed API - only scraping solutions exist. LinkedIn has official OAuth but the data APIs require third-party services like Proxycurl.

**Repositories** — pure data access, one per entity
- SourceRepository — CRUD, search by employer/project/date
- BulletRepository — CRUD, query by source, by status, by technology
- PerspectiveRepository — CRUD, query by bullet, by archetype, by framing
- ResumeRepository — CRUD, assemble/reorder perspectives, query coverage gaps
- All repositories use bun:sqlite (Bun's native SQLite driver — fast, zero-dependency)

**Services** — business logic, enforces the derivation chain rules
- SourceService — create/update sources, validate status transitions
- DerivationService — orchestrates Source→Bullet and Bullet→Perspective generation. Calls AI module, creates entities in pending_review status, enforces FK chain
- ResumeService — assembles resumes from approved perspectives, runs gap analysis (SQL-based skill coverage queries), recommends new perspective generation when gaps found
- AuditService — traces any perspective back through bullet→source, validates chain integrity

**AI Module** — isolated, swappable
- Wraps Anthropic SDK (Claude API)
- Structured prompts for each derivation type (source→bullet, bullet→perspective)
- Returns structured output (not freeform text) — the service validates and persists
- Stores the prompt used in generated_prompt field for auditability

**HTTP Routes** — thin layer mapping to services
- POST /sources, GET /sources/:id, PATCH /sources/:id, GET /sources
- POST /sources/:id/derive-bullets — triggers DerivationService
- POST /bullets/:id/derive-perspectives — triggers DerivationService
- PATCH /bullets/:id/approve, PATCH /bullets/:id/reject
- PATCH /perspectives/:id/approve, PATCH /perspectives/:id/reject
- POST /resumes, GET /resumes/:id/gaps, POST /resumes/:id/export
- Standard REST otherwise. JSON responses with typed error shapes.


Spec Review: Forge Resume Builder

  Suggested Parallelization (for implementation planning)

  Track A: Core (schema → repos → services → routes)     [critical path]
  Track B: SDK (once API surface locked)                  [blocks C, D]
  Track C: CLI (requires SDK)                             ──┐
  Track D: WebUI (requires SDK)                           ──┤ parallel
  Track E: Rust stubs (independent)                       ──┤
  Track F: Documentation (immediate)                      ──┘
