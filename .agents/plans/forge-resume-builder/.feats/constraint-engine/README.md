The constraint engine needs a relationship map — a declarative description of every entity relationship in Forge. Something like:

```typescript
const RELATIONSHIPS = {
  skills: {
    cascade: [
      // "when a skill is deleted, delete these"
      { table: 'bullet_skills', fk: 'skill_id' },
      { table: 'source_skills', fk: 'skill_id' },
      { table: 'jd_skills', fk: 'skill_id' },
      { table: 'resume_skills', fk: 'skill_id' },
      { table: 'skill_domains', fk: 'skill_id' },
      { table: 'summary_skills', fk: 'skill_id' },
      { table: 'certification_skills', fk: 'skill_id' },
      { table: 'perspective_skills', fk: 'skill_id' },
    ],
    restrict: [],  // no restrict rules on skills
    setNull: [],   // nothing set-nulls to skills
  },

  bullets: {
    cascade: [
      { table: 'bullet_sources', fk: 'bullet_id' },
      { table: 'bullet_skills', fk: 'bullet_id' },
    ],
    restrict: [
      // "cannot delete if these exist"
      { table: 'perspectives', fk: 'bullet_id' },
    ],
    setNull: [],
  },

  perspectives: {
    cascade: [
      { table: 'perspective_skills', fk: 'perspective_id' },
    ],
    restrict: [
      { table: 'resume_entries', fk: 'perspective_id' },
    ],
    setNull: [],
  },

  organizations: {
    cascade: [
      { table: 'org_tags', fk: 'organization_id' },
      { table: 'org_campuses', fk: 'organization_id' },
      { table: 'org_aliases', fk: 'organization_id' },
      { table: 'contact_organizations', fk: 'organization_id' },
    ],
    restrict: [],
    setNull: [
      { table: 'source_roles', fk: 'organization_id' },
      { table: 'source_projects', fk: 'organization_id' },
      { table: 'source_education', fk: 'organization_id' },
      { table: 'contacts', fk: 'organization_id' },
      { table: 'job_descriptions', fk: 'organization_id' },
      { table: 'credentials', fk: 'organization_id' },
      { table: 'certifications', fk: 'issuer_id' },
    ],
  },
  // ... every entity
}
```

What it does

Three operations on a delete(entityType, id) call:

1. RESTRICT check
    → For each restrict rule: count children
    → If any exist: reject with error (which children, how many)
    → Short-circuit — don't touch anything

2. CASCADE deletes
    → For each cascade rule: delete all rows where fk = id
    → Recursive: if a cascaded entity itself has cascade rules,
      follow them (e.g., delete resume → cascade sections →
      cascade entries + skills)

3. SET NULL updates
    → For each setNull rule: update rows SET fk = NULL where fk = id

4. Delete the entity itself

Key design questions

Q1: Does it handle recursive cascades?

Yes, it must. Deleting a resume cascades to resume_sections, which cascades to resume_entries and resume_skills. The engine needs to walk the tree:

delete resume
  → cascade resume_sections (by resume_id)
    → for each section:
      → cascade resume_entries (by section_id)
      → cascade resume_skills (by section_id)
      → cascade resume_certifications (by section_id)
  → cascade resume_certifications (by resume_id)
  → cascade job_description_resumes (by resume_id)
  → cascade contact_resumes (by resume_id)

This is a depth-first traversal of the relationship graph. The map needs to know that resume_sections is also an entity with its own cascade rules.

Q2: Does it wrap everything in a transaction?

Yes. The entire cascade tree must be atomic. The engine calls adapter.beginTransaction(), does all the work, then adapter.commit(). If anything fails, adapter.rollback().

Q3: Does it also handle creates and updates?

For deletes it's essential. For creates/updates the question is:
- FK existence validation (does the referenced parent exist?) — this is a flavor of RESTRICT in reverse. Currently services check this. Could the constraint engine also handle "before creating source_role, verify organization_id
exists"?
- Unique constraint checking — "before creating skill, verify name doesn't exist"

If yes, the engine becomes a full integrity layer, not just a delete coordinator. The API might look like:

interface ConstraintEngine {
  // Delete with full cascade/restrict/setNull
  delete(entityType: string, id: string): Promise<Result>

  // Validate before write (FK existence + uniqueness)
  validateCreate(entityType: string, data: object): Promise<Result>
  validateUpdate(entityType: string, id: string, data: object): Promise<Result>
}

Q4: How does it interact with the adapter?

The engine sits between the service and the adapter. It uses the adapter for raw CRUD:

Service
  → ConstraintEngine.delete('skills', id)
    → adapter.count('perspectives', { bullet_id: id })  // restrict check
    → adapter.deleteWhere('bullet_skills', { skill_id: id })  // cascade
    → adapter.deleteWhere('source_skills', { skill_id: id })  // cascade
    → adapter.delete('skills', id)  // actual delete

The adapter's contract is simple: read, write, delete, count, updateWhere. No relational awareness.

Q5: What about the SQLite adapter specifically?

Two options:
- Let it double-cascade. Constraint engine cascades, then SQLite DB also cascades. Harmless — the DB cascade finds nothing to do because the engine already cleaned up. Slightly wasteful but correct.
- SQLite adapter skips engine. Register the SQLite adapter as "FK-capable" and let the engine pass through to the DB's native FK handling. Cleaner but adds a capability flag.

I'd lean toward letting it double-cascade for simplicity. The cost is negligible — a few DELETE WHERE clauses that affect zero rows.

Q6: Where does the relationship map come from?

Three options:
- Hand-maintained TypeScript object — what I showed above. Clear, readable, but must stay in sync with schema.
- Generated from migrations — a build step that parses SQL migrations and extracts FK rules. Auto-sync but adds build complexity.
- Introspected at runtime — PRAGMA foreign_key_list(table) for SQLite. Only works for SQLite, not useful for other backends.

I'd start with hand-maintained + a test that validates it against the actual SQLite schema via PRAGMA introspection. The test catches drift.

---
What's your reaction? Does this shape feel right, or does the scope concern you — especially around Q3 (should it also handle create/update validation)?
