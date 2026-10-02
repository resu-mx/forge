# Prompt Template: Source → Bullet Derivation

## Template

```
You are a resume content assistant. Given a source description of work performed,
decompose it into factual bullet points. Each bullet must:
- State only facts present in the source description
- Include specific technologies, tools, or methods mentioned
- Include quantitative metrics if present in the source
- NOT infer, embellish, or add context not explicitly stated

Source description:
---
{source.description}
---

Respond with a JSON object:
{
  "bullets": [
    {
      "content": "factual bullet text",
      "technologies": ["tech1", "tech2"],
      "metrics": "quantitative metric if present, null otherwise"
    }
  ]
}
```

## Example Input

```
Led a team of 4 engineers to migrate a cloud forensics platform from
on-prem ELK to AWS OpenSearch. The migration took 6 months and reduced mean
incident response time by 40%. Used Terraform for infrastructure, GitLab CI/CD
for deployment automation, and built custom Python log parsers for format
translation.
```

## Example Output

```json
{
  "bullets": [
    {
      "content": "Led 4-engineer team migrating cloud forensics platform from ELK to AWS OpenSearch over 6 months",
      "technologies": ["ELK", "AWS OpenSearch"],
      "metrics": "4 engineers, 6 months"
    },
    {
      "content": "Reduced mean incident response time by 40% through platform migration",
      "technologies": [],
      "metrics": "40% reduction in MTTR"
    },
    {
      "content": "Built infrastructure automation with Terraform and GitLab CI/CD for deployment pipeline",
      "technologies": ["Terraform", "GitLab CI/CD"],
      "metrics": null
    },
    {
      "content": "Developed custom Python log parsers for format translation between ELK and OpenSearch",
      "technologies": ["Python"],
      "metrics": null
    }
  ]
}
```

## Notes

- These templates are starting points. They will be refined through use.
- The prompt is stored in `prompt_logs.prompt_input` for auditability.
- Template version should be tracked in `prompt_logs.prompt_template` (e.g., "source-to-bullet-v1").
