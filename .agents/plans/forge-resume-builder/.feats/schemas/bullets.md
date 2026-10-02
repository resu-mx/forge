# Bullets

## **Bullet Types**

- Story Bullets: These live in `Source` Objects
  - Tactical Scope
  - Operational Scope
  - Strategic Scope
- Derived Bullets: Use GraphRAG to provide context for deriving these `Intermediate` Bullets.
  - *Context*: Use Story Bullets
  - *Goal*: Provide High-Quality, structured bullets for use in Resumes/Perspectives
- Perspective Bullets: These are Variant Bullets, for specific perspectives.
  - *Context*: Use Derived Bullets and JD Roles+Reqs
  - *Goal*: Provide Tuned, Opinionated, structured bullets for a specific JD, Resume, or Archetype/Domain

## **Theme**: represent the "Why" (long-term)

- Definition: A high-level, overarching/long-term strategic goals or areas of focus for the organization, often spanning months
- Purpose: Helps stakeholders prioritize, group, and track strategic initiatives (e.g., "User Onboarding Experience," "Mobile First Initiative").
- Scope: Contains multiple Epics and User Stories.
- Ex: "Improve Checkout"

## **Story**: represent the "What" (short-term)

- Definition: A short, simple/actionable requirement written from the user's perspective, representing a single piece of functionality.
- Purpose: To define, build, and test a specific function in a single sprint.
- Ex: "Add credit card field", "As a user, I want to reset my password, so I can regain access"


## Schema: `Bullet::Story`

> Q: How to measure/record/annotate TechnialDetail vs BusinessImpact vs outcome-focused
> Q: How to measure Specificity vs Vagueness
> Q: Thoughts on storing 'perspectives' as git-change of sourced 'bullet'?

```json
{
  // TODO: Design Schema for `Measure`
  "action": {
    "verbs": ["<String>"]
  },
  "context": "", // Describes Context that drives the <Story>
  "result": "", // Should Address <Story>
  "measure": "<Measure>", // Describes Measures for Success/Failure
  "work": "<DateRange>", // TimeCost
  "skills": ["<ref::<Skill>>"],
  "url": "<url>",
  "domain": "<ref::<Domain>>",
  "source": "<ref::<Source::*>>",
  "problem": "<String>", // Describes Need for the <Story>
}
```
