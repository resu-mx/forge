# End-to-End Derivation Chain Example

## Step 1: Create Source (Human)

```
POST /sources
{
  "title": "Cloud Forensics Platform Migration",
  "description": "Led a team of 4 engineers to migrate a cloud forensics platform from on-prem ELK to AWS OpenSearch. The migration took 6 months and reduced mean incident response time by 40%. Used Terraform for infrastructure, GitLab CI/CD for deployment automation, and built custom Python log parsers for format translation.",
  "employer_id": "acme-corp-uuid",
  "start_date": "2023-01-15",
  "end_date": "2023-07-15"
}
```

Source created with status `draft`. Human marks as `approved` when satisfied.

## Step 2: Derive Bullets (AI + Human Review)

```
POST /sources/{source-id}/derive-bullets
```

AI produces 4 bullets, all in `pending_review` status:

1. "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months" — technologies: [ELK, AWS OpenSearch], metrics: "4 engineers, 6 months"
2. "Reduced mean incident response time by 40% through platform migration" — metrics: "40% MTTR reduction"
3. "Built infrastructure automation with Terraform and GitLab CI/CD for deployment pipeline" — technologies: [Terraform, GitLab CI/CD]
4. "Developed custom Python log parsers for format translation between ELK and OpenSearch" — technologies: [Python]

Each bullet has `source_content_snapshot` = the source's description at this moment.

Human reviews:
- Approves bullets 1, 2, 3
- Rejects bullet 4 with reason: "This was a minor script, not a development effort — overstates scope"

## Step 3: Derive Perspectives (AI + Human Review)

```
POST /bullets/{bullet-1-id}/derive-perspectives
{ "archetype": "agentic-ai", "domain": "ai_ml", "framing": "accomplishment" }
```

AI produces:
> "Led cloud platform migration enabling ML-based log analysis pipeline on AWS OpenSearch"

`bullet_content_snapshot` = bullet 1's content at this moment. Status: `pending_review`.

Human approves.

Repeat for other archetype/domain/framing combinations as needed.

## Step 4: Assemble Resume

```
POST /resumes
{ "name": "AI Engineer - Target Co", "target_role": "AI Engineer", "target_employer": "Target Co", "archetype": "agentic-ai" }

POST /resumes/{resume-id}/perspectives
{ "perspective_id": "perspective-uuid", "section": "work_history", "position": 1 }
```

## Step 5: Check Gaps

```
GET /resumes/{resume-id}/gaps
```

Returns domains not covered, bullets with no perspectives for this archetype, etc.

## Chain Provenance at Any Point

```
GET /perspectives/{perspective-id}
```

Returns:
```json
{
  "data": {
    "id": "perspective-uuid",
    "content": "Led cloud platform migration enabling ML-based log analysis pipeline on AWS OpenSearch",
    "bullet_content_snapshot": "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months",
    "bullet": {
      "id": "bullet-uuid",
      "content": "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months",
      "source_content_snapshot": "Led a team of 4 engineers to migrate Acme Corp's...",
      "source": {
        "id": "source-uuid",
        "title": "Cloud Forensics Platform Migration",
        "description": "Led a team of 4 engineers to migrate Acme Corp's..."
      }
    }
  }
}
```

Snapshots match current content → chain is clean.
