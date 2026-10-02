# Logic Distribution Audit — Where Things Live Today

**Purpose:** Inform the storage adapter boundary decision.

## Layer Summary

| Layer | Responsibility Count | Character |
|---|---|---|
| **DB (SQLite)** | 60+ CHECKs, 80+ FKs, 35+ UNIQUEs, 1 trigger, STRICT on all tables | Enforces structure, enums, referential integrity, type safety |
| **Repositories (28)** | JOIN composition, data transformation, junction CRUD, transaction wrappers | Translates between SQL and domain objects |
| **Services (26)** | Validation, status FSM, cross-entity coordination, side effects, computed data | Business rules, orchestration, error handling |
| **Routes** | Input parsing, HTTP concerns | Thin — almost no logic |

---

## Category 1: Enum / Type Constraints

**Current location:** DB (CHECK constraints) + Service (validation before write)

| What | DB enforces | Service enforces |
|---|---|---|
| Status values (bullets, perspectives, sources, resumes, JDs, credentials) | CHECK IN (...) | Validates + enforces transition FSM (VALID_TRANSITIONS map) |
| Entity type enums (source_type, org_type, credential_type, education_type, etc.) | CHECK IN (...) | Validates before write |
| Relationship enums (contact relationships, presentation_type, etc.) | CHECK IN (...) | Validates before write |
| Skill category | FK to skill_categories(slug) | Validates against enum list |

**Tension:** Double validation — service checks, then DB checks again. Services don't trust the DB; DB doesn't trust the services.

**Backend impact:**
- HelixDB: Compile-time schema types replace CHECK — stronger guarantee, but enum changes require schema migration
- GraphQLite: No schema enforcement — ALL validation moves to adapter/service
- DuckPGQ: Read-only — validation irrelevant

---

## Category 2: Referential Integrity (FK Cascades)

**Current location:** DB (ON DELETE CASCADE/RESTRICT/SET NULL)

### CASCADE (cleanup junctions on parent delete) — 40+ rules
All junction tables CASCADE from both sides. Extension tables CASCADE from parent.
Examples: bullet_sources, skill_domains, resume_sections, org_tags, contact_*, etc.

### RESTRICT (prevent delete if children exist) — 2 critical rules
- `perspectives.bullet_id → bullets(id) ON DELETE RESTRICT` — can't delete bullet with perspectives
- `resume_entries.perspective_id → perspectives(id) ON DELETE RESTRICT` — can't delete perspective used in resume

### SET NULL (detach on parent delete) — 15+ rules
- org FKs on sources, JDs, contacts, credentials, certifications
- summary_id on resumes
- prompt_log_id on bullets/perspectives

**Tension:** Services ALSO check before delete (e.g., archetype service counts references, industry service counts references). The DB RESTRICT is a safety net, not the primary mechanism.

**Backend impact:**
- HelixDB: No SQL-style CASCADE. Deleting a node does NOT auto-delete edges. Adapter must implement cascade logic explicitly. RESTRICT must be checked in code.
- GraphQLite: Inherits SQLite FK behavior (if using same DB file). But graph edges in EAV tables are separate — deleting a relational row doesn't clean up graph edges.
- DuckPGQ: Read-only — N/A.

**This is the hardest category.** HelixDB requires the adapter to reimplement ~40 cascade rules and 2 restrict rules.

---

## Category 3: Uniqueness Enforcement

**Current location:** DB (UNIQUE constraints, composite PKs)

### Entity uniqueness
- skills.name, domains.name, archetypes.name, industries.name, role_types.name, skill_categories.slug

### Junction uniqueness (prevents duplicate links)
- 15+ composite PKs on junction tables (e.g., bullet_skills PK(bullet_id, skill_id))
- resume_skills UNIQUE(section_id, skill_id)
- resume_certifications UNIQUE(resume_id, certification_id)
- embeddings UNIQUE(entity_type, entity_id)
- pending_derivations UNIQUE INDEX(entity_type, entity_id) — concurrency lock

### Case-insensitive uniqueness
- org_aliases.alias COLLATE NOCASE

**Backend impact:**
- HelixDB: HQL schemas can enforce uniqueness per node type. Edge uniqueness (no duplicate edges between same nodes) needs explicit handling.
- GraphQLite: Schemaless — no uniqueness enforcement. Must check in adapter code.
- DuckPGQ: Read-only — N/A.

---

## Category 4: Data Transformation (Read Path)

**Current location:** Repository layer

| Transformation | Where | Examples |
|---|---|---|
| Boolean conversion (0/1 ↔ true/false) | Repository | credentials.in_progress, orgs.worked, source_projects.open_source |
| JSON parsing | Repository | credentials.details, templates.sections, resumes.header |
| Array aggregation | Repository | bullet→technologies (via bullet_skills), cert→skills |
| Nested object hydration | Repository | perspective.getWithChain (3-table deep), resume.getWithEntries |
| Computed fields (subqueries) | Repository | summary.linked_resume_count, archetype.usage_counts |
| Content resolution (COALESCE) | Repository | resume_entry.content ?? perspective.content |

**Backend impact:** This is pure read-side transformation. Stays in the adapter regardless of backend. Each adapter maps its native response format to the domain object. This is straightforward.

---

## Category 5: Write-Side Logic

**Current location:** Split between Repository and Service

### Repository-level write logic
- **Upsert patterns:** Embedding.upsert (INSERT ON CONFLICT), Skill.getOrCreate (INSERT OR IGNORE + SELECT)
- **Conditional writes:** Source.acquireDerivingLock (UPDATE WHERE status != 'deriving')
- **Multi-statement writes:** Bullet.addSource (demote existing primary → promote new), Bullet.update (DELETE old tech → INSERT new)
- **Batch reorder:** Resume.reorderEntries, Resume.reorderSkills (loop UPDATE in transaction)
- **Snapshot capture:** ResumeEntry.create/update (capture perspective_content_snapshot if perspective_id set)

### Service-level write logic
- **Status FSM:** Bullet/Perspective approve/reject/reopen with transition validation
- **Computed values:** source_content_snapshot, SHA256 hashes, TF-IDF taglines
- **Cross-entity atomic writes:** Template.createResumeFromTemplate (resume + sections in tx), Derivation.commit (bullets + prompt_log + lock delete in tx)
- **Fire-and-forget side effects:** Embedding hooks on bullet/perspective/source/JD create (queueMicrotask)

**Backend impact:**
- Upsert patterns: HelixDB has different write semantics (node upsert vs INSERT ON CONFLICT). Adapter must abstract.
- Conditional writes: HelixDB doesn't have SQL WHERE-clause conditional updates. Must read-then-write or use HQL equivalent.
- Transactions: HelixDB has ACID (LMDB), but transaction API differs. Adapter must provide transaction abstraction.
- Snapshot capture: Pure application logic — stays in service regardless.

---

## Category 6: Cross-Entity Coordination

**Current location:** Service layer (with repository JOINs)

### Read coordination (queries spanning multiple entities)
- **Resume compiler:** resumes → sections → entries → perspectives → bullets → sources + orgs (6+ tables)
- **Review service:** pending bullets/perspectives with full chain context
- **Audit/integrity service:** snapshot drift detection across derivation chain
- **Alignment service:** JD requirements vs resume entries via embedding similarity

### Write coordination (atomic multi-entity mutations)
- **Template → Resume creation:** create resume + clone sections (transaction)
- **Derivation commit:** write bullets/perspectives + prompt_log + delete lock (transaction)
- **Source creation:** write source + extension table (transaction)

**Backend impact:**
- Read coordination: Graph DBs excel here — single traversal query vs multi-JOIN. This is the primary benefit.
- Write coordination: Must maintain atomicity. HelixDB ACID handles this, but API differs from SQLite transactions.

---

## Category 7: Side Effects & Async Operations

**Current location:** Service layer

- **Embedding hooks:** 4 services fire-and-forget embed on create/update (queueMicrotask)
- **Export:** shells out to sqlite3, tectonic
- **Stale lock recovery:** runs at server startup

**Backend impact:**
- Embedding hooks: In HelixDB, vector is a native property — embedding might happen at write time, not as a side effect. Fundamentally different model.
- Export: sqlite3 dump is SQLite-specific. Each adapter needs its own export implementation.
- Lock recovery: pending_derivations cleanup is backend-specific.

---

## The Boundary Question: What Pattern Emerges?

### Logic that MUST move into the adapter (backend-specific)
1. **Cascade/Restrict rules** — each backend implements differently
2. **Uniqueness enforcement** — different mechanisms per backend
3. **Upsert/conditional write patterns** — SQL-specific idioms
4. **Transaction API** — each backend has its own
5. **Query composition** — JOINs vs traversals vs pattern matching
6. **Data serialization** — JSON blobs, binary vectors, boolean mapping

### Logic that SHOULD stay in services (backend-agnostic)
1. **Status FSM transitions** — pure business rule
2. **Input validation** — field format, enum membership, cross-field rules
3. **Computed values** — snapshots, hashes, TF-IDF, cosine similarity
4. **Side effect orchestration** — embedding hooks, export coordination
5. **Error mapping** — constraint violations → domain errors

### Logic that's AMBIGUOUS (the hard decisions)
1. **FK existence checks before write** — service currently checks org exists before creating source_role. Does the adapter guarantee this or does the service keep checking?
2. **Enum enforcement** — currently double-checked (service + DB). Who owns it?
3. **Embedding lifecycle** — side effect in SQLite, native property in HelixDB. Where's the boundary?
4. **Reference counting before delete** — service checks "has perspectives?" before deleting bullet. Is this a storage concern (RESTRICT) or business logic?
5. **Snapshot capture on write** — repository currently captures perspective_content_snapshot. Is this a storage hook or service responsibility?
