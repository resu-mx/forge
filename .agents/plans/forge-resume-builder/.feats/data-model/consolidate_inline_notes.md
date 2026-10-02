# Consolidate Inline Notes to user_notes Table

## Problem
8 tables have inline `notes TEXT` fields (sources, bullets, perspectives, resumes, resume_entries, skills, organizations, job_descriptions). This means searching "all notes" requires querying 8 tables + user_notes. Notes are fragmented across the schema.

## Proposal
Remove inline `notes TEXT` columns from all entity tables. All notes should live in `user_notes` and be linked via `note_references(note_id, entity_type, entity_id)`.

## Benefits
- Single search surface for all notes
- Notes can reference multiple entities (already supported)
- Consistent UI pattern — one "notes" panel, not per-entity text fields
- Simpler schema — fewer nullable TEXT columns

## Migration Path
1. For each table with inline `notes`, create `user_notes` entries from non-null values
2. Create `note_references` linking each migrated note to its source entity
3. Drop `notes TEXT` columns from entity tables
4. Update routes/services/MCP tools to use note_references instead of inline field
5. Update WebUI note editing to use shared notes component

## Considerations
- `note_references` entity_type CHECK constraint already covers all 8 entity types (plus credential, certification)
- NoteService VALID_ENTITY_TYPES needs to add 'credential' and 'certification' (currently missing)
- MCP tools for note management don't exist yet — would need `forge_create_note`, `forge_search_notes`, `forge_link_note`, `forge_unlink_note`
