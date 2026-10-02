# Content Versioning Strategy

## Problem

When a Source is edited after Bullets have been derived from it, the derived Bullets' content was based on the old Source text. The chain of custody is silently broken.

## MVP Solution: Content Snapshots

Instead of full version history, capture the content at each derivation point.

### Fields

- `Bullet.source_content_snapshot` — Source.description at the time this bullet was derived
- `Perspective.bullet_content_snapshot` — Bullet.content at the time this perspective was derived

### Chain Integrity Check

For any Perspective, the system can compare:
```
perspective.bullet_content_snapshot  vs  bullet.content       (drift at bullet level)
bullet.source_content_snapshot       vs  source.description   (drift at source level)
```

If they match → chain is clean (green in UI)
If they diverge → chain has drifted (yellow warning in UI)

### UI Behavior

The Chain View shows:
- Current content of each entity
- The snapshot stored at derivation time
- A visual diff highlight when snapshot != current

This lets the user see exactly what changed and decide whether the derived content is still valid.

### What This Does NOT Do

- No automatic invalidation of derived content when upstream changes
- No version history (only one snapshot per derivation)
- No "re-derive" trigger on upstream changes

These are deliberate MVP scope limits. The user manually reviews drift and re-derives if needed.

## Post-MVP: Full Version History

A future enhancement could add a `content_versions` table:
```sql
CREATE TABLE content_versions (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  content TEXT NOT NULL,
  changed_by TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

This would capture every edit, not just derivation-time snapshots.
