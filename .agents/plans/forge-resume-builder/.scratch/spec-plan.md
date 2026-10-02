```markdown
Create [SPECs](.claude/plans/forge-resume-builder/refs/specs/) for
```


```markdown
Use subagents to Review each of the following SPEC Individually, to look for GAPs, inconsistencies, errors, oversights, refinements, enhancements, acceptance criteria, context, test cases, suggested parallelizations, dependencies, and examples.
- Categorize findings into [CRITICAL, SEVERE, IMPORTANT, MINOR, GAP, INCONSISTENCY, ANTI-PATTERN, ENHANCEMENT] 
- SPECs
    - .claude/plans/forge-resume-builder/refs/specs/2026-04-03-mcp-server-design.md
```




```markdown
Use subagents to write a SPEC aligned, Phased, dependency aware, and parallelization aware (where reasonable) plan to .claude/plans/forge-resume-builder/*
- Categorize findings into [CRITICAL, SEVERE, IMPORTANT, MINOR, GAP, INCONSISTENCY, ANTI-PATTERN, ENHANCEMENT]
- Each phase plan should include 
    - examples, 
    - acceptance/failure criteria
    - goals, non-goals
    - fall back strategies, context
    - documenation requirements
    - testing support
        - fixtures/test-cases
        - Test Kinds
            - unit, integration, e2e, component, smoke, contract, doc tests, visual tests
- Update the existing plan to include new phases
- Create the phase plans in parallel (where reasonable)
- SPECs
    - .claude/plans/forge-resume-builder/refs/specs/2026-04-03-mcp-server-design.md
```



```markdown
Use subagents to Review new Plans at .claude/plans/forge-resume-builder/**
- look for GAPs, inconsistencies, errors, oversights, refinements, enhancements, acceptance criteria, context, test cases, suggested parallelizations, dependencies, and examples.
- Categorize findings into [CRITICAL, SEVERE, IMPORTANT, MINOR, GAP, INCONSISTENCY, ANTI-PATTERN, ENHANCEMENT]
- Does the plan clearly state what can be parallelized and what the dependency chain is?
- Does the plan clearly state when, where, and what to document?
```
