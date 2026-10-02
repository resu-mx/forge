# Job Description

## Schema: `JobDescription`

```json
{
  "title": "<String>",
  "organization": "<ref::<Organization>>",
  "status": "<JDStatus>",
  "salary": {
    "low": "<Int>",
    "high": "<Int>",
  },
  "url": "<url>",
  "domain": "<ref::<Domain>>",
  "skills": ["<ref::<Skill>>"],
  "role": {
    "about": "<String>",
    "responsibilities": ["<Responsibilities>"],
    "requirements": ["<Requirements::Hard>"],
    "qualifications": ["<Qualifications>"],
    "preferences": ["<Requirements::Bonus>"],
  }
  "resumes": ["<ref::<Resume>>"],
  "contacts": [{"<ref::<Contact::Relationship>>": "<ref::<Contact>>"}], // Grouped lists of <Relation>:<Contact>
}
```
