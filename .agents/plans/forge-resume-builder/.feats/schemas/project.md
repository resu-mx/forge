## Schema: `Project`

```json
{
  "product": "<ref::<Product>>",
  "title": "<String>", // Describes <Theme> (https://www.visual-paradigm.com/scrum/theme-epic-user-story-task/)
  "bullets": ["<Bullet::Story>"],
  "domain": "<ref::<Domain>>",
  "url": "<url>",
  "skills": ["<ref::<Skill>>"],
  "work": "<DateRange>", // TimeCost
  "outcome": "<String>", // Should Address <Theme>
  "situation": "<String>", // Describes Context that drives the <Theme>
  "problem": "<String>", // Describes Need for the <Theme>
  "measure": "<Measure>", // Describes Measures for Success/Failure
}
```
