# Tagging Strategy

## Technology Tags (Bullet Technologies)

Technologies are stored in a junction table `bullet_technologies` for efficient querying.

### Creation
When bullets are created (either manually or via AI derivation), technologies are:
1. Extracted from the AI response's `technologies` array
2. Inserted into `bullet_technologies` as `(bullet_id, technology)` pairs
3. Technology names are lowercased and trimmed for consistency

### Querying
```sql
-- All bullets mentioning a technology
SELECT b.* FROM bullets b
JOIN bullet_technologies bt ON b.id = bt.bullet_id
WHERE bt.technology = 'kubernetes';

-- All technologies for a bullet
SELECT technology FROM bullet_technologies WHERE bullet_id = ?;

-- Technology frequency (most used)
SELECT technology, COUNT(*) as count
FROM bullet_technologies
GROUP BY technology
ORDER BY count DESC;
```

### API Response
Bullet API responses include technologies as a denormalized array:
```json
{
  "id": "uuid",
  "content": "...",
  "technologies": ["terraform", "gitlab ci/cd", "aws"]
}
```

This is assembled from the junction table at query time, not stored as JSON.

## Skill Tags (Bullet Skills, Perspective Skills)

Skills are normalized entities with their own table. Junction tables link skills to bullets and perspectives.

### Creation
Skills are managed separately from technologies:
- Technologies are free-text tags extracted from AI output
- Skills are curated entities managed via the Skills API (future)
- For MVP, skills are manually associated via the API

### Querying
```sql
-- Perspectives covering a skill
SELECT p.* FROM perspectives p
JOIN perspective_skills ps ON p.id = ps.perspective_id
WHERE ps.skill_id = ?;

-- Skills coverage for a resume
SELECT DISTINCT s.name FROM skills s
JOIN perspective_skills ps ON s.id = ps.skill_id
JOIN resume_perspectives rp ON ps.perspective_id = rp.perspective_id
WHERE rp.resume_id = ?;
```
