# Database Indexing Strategy

## Index Design Principles

1. Index columns used in WHERE clauses of common queries
2. Index FK columns for JOIN performance
3. Index status columns for filtered list queries
4. Composite indexes for common multi-column filters

## Indexes

### Sources
| Index | Columns | Purpose |
|---|---|---|
| `idx_sources_status` | `status` | Filter by draft/approved/deriving |
| `idx_sources_employer` | `employer_id` | Filter/join by employer |
| `idx_sources_project` | `project_id` | Filter/join by project |

### Bullets
| Index | Columns | Purpose |
|---|---|---|
| `idx_bullets_source` | `source_id` | List bullets for a source |
| `idx_bullets_status` | `status` | Filter by pending_review/approved/etc |

### Bullet Technologies
| Index | Columns | Purpose |
|---|---|---|
| PK | `(bullet_id, technology)` | Composite primary key |
| `idx_bullet_tech_technology` | `technology` | "All bullets with Kubernetes" query |

### Perspectives
| Index | Columns | Purpose |
|---|---|---|
| `idx_perspectives_bullet` | `bullet_id` | List perspectives for a bullet |
| `idx_perspectives_status` | `status` | Filter by status |
| `idx_perspectives_archetype` | `target_archetype` | Filter by archetype |
| `idx_perspectives_domain` | `domain` | Filter by domain |

### Resume Perspectives
| Index | Columns | Purpose |
|---|---|---|
| `idx_resume_perspectives_resume` | `(resume_id, section, position)` | Ordered retrieval of resume content |

### Prompt Logs
| Index | Columns | Purpose |
|---|---|---|
| `idx_prompt_logs_entity` | `(entity_type, entity_id)` | Look up prompts for a specific bullet/perspective |

## Query Patterns

Most common queries and their index usage:

```sql
-- List pending review items (review queue)
SELECT * FROM bullets WHERE status = 'pending_review';  -- uses idx_bullets_status
SELECT * FROM perspectives WHERE status = 'pending_review';  -- uses idx_perspectives_status

-- Bullets for a source
SELECT * FROM bullets WHERE source_id = ?;  -- uses idx_bullets_source

-- Perspectives for an archetype
SELECT * FROM perspectives WHERE target_archetype = ? AND status = 'approved';
-- uses idx_perspectives_archetype, then filters on status

-- Technology search
SELECT b.* FROM bullets b
JOIN bullet_technologies bt ON b.id = bt.bullet_id
WHERE bt.technology = ?;  -- uses idx_bullet_tech_technology
```
