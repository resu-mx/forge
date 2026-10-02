# Gap Analysis Algorithm

## Inputs

- `resume_id` — identifies the resume and its archetype, target_role, and included perspectives

## Algorithm

```
function analyzeGaps(resumeId):
  resume = getResume(resumeId)
  includedPerspectives = getResumePerspectives(resumeId)
  includedPerspectiveIds = Set(includedPerspectives.map(p => p.perspective_id))

  // 1. Get all approved perspectives for this archetype
  allForArchetype = perspectives.list({
    target_archetype: resume.archetype,
    status: 'approved'
  })

  // 2. Compute domain coverage from included perspectives
  includedDomains = Map<domain, count>
  for p in includedPerspectives:
    if p.domain:
      includedDomains[p.domain] += 1

  // 3. Define expected domains for the archetype
  expectedDomains = getExpectedDomains(resume.archetype)
  // e.g., agentic-ai expects: ai_ml, software_engineering, leadership

  // 4. Find missing domains
  gaps = []
  for domain in expectedDomains:
    if domain not in includedDomains:
      // Find bullets that could fill this gap
      // Approved bullets that have NO approved perspective for this archetype+domain
      availableBullets = db.query(`
        SELECT b.* FROM bullets b
        WHERE b.status = 'approved'
        AND b.id NOT IN (
          SELECT p.bullet_id FROM perspectives p
          WHERE p.target_archetype = ?
          AND p.domain = ?
          AND p.status = 'approved'
        )
      `, [resume.archetype, domain])
      gaps.push({
        type: "missing_domain_coverage",
        domain: domain,
        available_bullets: availableBullets,
        recommendation: "Derive perspectives with domain '{domain}'"
      })
    elif includedDomains[domain] < THIN_THRESHOLD:
      gaps.push({
        type: "thin_coverage",
        domain: domain,
        current_count: includedDomains[domain],
        recommendation: "Consider adding more {domain} perspectives"
      })

  // 5. Find unused bullets (approved bullets with no perspective for this archetype)
  allApprovedBullets = bullets.list({ status: 'approved' })
  for bullet in allApprovedBullets:
    perspectivesForBullet = perspectives.list({
      bullet_id: bullet.id,
      target_archetype: resume.archetype,
      status: 'approved'
    })
    if perspectivesForBullet.length == 0:
      gaps.push({
        type: "unused_bullet",
        bullet_id: bullet.id,
        recommendation: "Derive perspective for archetype '{resume.archetype}'"
      })

  // 6. Compute coverage summary
  coveredSkills = unique skills from included perspectives (via perspective_skills)
  domainsRepresented = unique domains from included perspectives
  domainsMissing = expectedDomains - domainsRepresented

  return { resume_id, archetype, gaps, coverage_summary }
```

## Constants

- `THIN_THRESHOLD = 2` — fewer than 2 perspectives in a domain = thin coverage

## Expected Domains by Archetype

| Archetype | Expected Domains |
|---|---|
| `agentic-ai` | ai_ml, software_engineering, leadership |
| `infrastructure` | systems_engineering, devops, software_engineering |
| `security-engineer` | security, systems_engineering, devops |
| `solutions-architect` | systems_engineering, software_engineering, leadership |
| `public-sector` | security, systems_engineering, leadership |
| `hft` | systems_engineering, software_engineering |

These mappings are configurable and stored as application constants (not in the database for MVP).
