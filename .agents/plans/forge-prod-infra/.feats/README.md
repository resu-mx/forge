
## Workflows

1. HTML
2. Readability/trafilatura first (extract article content)
3. markdown-it
4. clean text
5. Crawl4AI which does content extraction built-in
6. GLiNER2 (single pass, 4 tasks):
    - entities:        [skills, certs, clearance, company, location, salary]
    - classification:  [remote/hybrid/onsite/unclear, seniority, IC/management]
    - structured:      [requirements_required, requirements_preferred]
    - relations:       [skill→required/preferred]
7. JSON
8. Forge ingest


## Frictions

```
1. The "required vs preferred" problem. JDs express this in wildly inconsistent ways:

## Requirements
- 5+ years Python          ← clearly required
- AWS experience a plus    ← clearly preferred

## Qualifications
- Must have active TS/SCI  ← required, different section
- Familiarity with K8s     ← ambiguous

GLiNER2 can handle this with labels like requirement_required and requirement_preferred in the entity task — but the quality depends heavily on the label schema you give it. The nice thing is GLiNER2 is zero-shot for entity extraction, so you can iterate on labels without retraining.
```

## Dedup/Caching Layer

At 50K/day you'll re-encounter the same JD from multiple job boards. Hash the cleaned markdown before inference — if you've seen it, skip GLiNER2 and return the cached extraction. This alone probably cuts your actual inference volume 60-80%.

**A concrete question**: Do you want to explore this as a new Forge subsystem (a `packages/extractor` or `packages/crawler` package), or as a separate service that feeds into Forge via API? The answer affects whether this is part of the ast-grep work we just planned or a separate epic.

## Architecture

```asciidoc
┌─────────────────────────┐     ┌──────────────────────────┐
│  Forge (Open Source)    │     │  Extractor Service       │
│  - Resume builder       │────▶│  - GLiNER2 inference     │
│  - MCP server           │ API │  - Crawl orchestration   │
│  - WebUI                │◀────│  - Dedup/cache layer     │
│                         │     │  - Structured JD output  │
│  Local: Bun + SQLite    │     │                          │
│  SaaS: CF Workers + D1  │     │  Local: Docker container │
└─────────────────────────┘     │  SaaS: CF Container / HF │
                                └──────────────────────────┘
```
**Local mode**: User runs the extractor as a Docker container (or Python process). Forge calls `http://localhost:PORT/extract`.

**SaaS mode**: You host the extractor. Forge SaaS calls the managed endpoint. Same API contract.
