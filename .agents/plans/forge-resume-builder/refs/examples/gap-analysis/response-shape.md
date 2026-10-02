# Gap Analysis Response Shape

## Endpoint

`GET /resumes/:id/gaps`

## Example Response

```json
{
  "data": {
    "resume_id": "resume-uuid",
    "archetype": "agentic-ai",
    "target_role": "AI Engineer",
    "target_employer": "Anthropic",
    "gaps": [
      {
        "type": "missing_domain_coverage",
        "domain": "ai_ml",
        "description": "No approved perspectives with domain 'ai_ml' are included in this resume",
        "available_bullets": [
          {
            "id": "bullet-uuid-1",
            "content": "Led 4-engineer team migrating cloud forensics platform...",
            "source_title": "Cloud Forensics Platform Migration"
          }
        ],
        "recommendation": "Derive perspectives with domain 'ai_ml' from these bullets"
      },
      {
        "type": "thin_coverage",
        "domain": "leadership",
        "current_count": 1,
        "description": "Only 1 perspective with domain 'leadership' — consider adding more",
        "recommendation": "Review approved bullets for additional leadership framing opportunities"
      },
      {
        "type": "unused_bullet",
        "bullet_id": "bullet-uuid-3",
        "bullet_content": "Built infrastructure automation with Terraform and GitLab CI/CD...",
        "source_title": "Cloud Forensics Platform Migration",
        "description": "This approved bullet has no perspective for archetype 'agentic-ai'",
        "recommendation": "Derive a perspective targeting 'agentic-ai' archetype"
      }
    ],
    "coverage_summary": {
      "perspectives_included": 8,
      "total_approved_perspectives_for_archetype": 15,
      "skills_covered": ["python", "aws", "kubernetes", "terraform"],
      "domains_represented": ["software_engineering", "devops"],
      "domains_missing": ["ai_ml", "leadership"]
    }
  }
}
```
