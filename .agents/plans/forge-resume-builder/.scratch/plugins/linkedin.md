Indeed job URL: https://www.indeed.com/viewjob?jk=c497c6e99dc69a7e
jk=c497c6e99dc69a7e

TODO:
- linkedin-mcp server
    - FEAT_REQ: Add support for show Saved Jobs
        - export_saved_jobs
        - list_saved_jobs
        - save_job
        - remove_saved_job
- How to 'Parse job responsibilities into a normalized+queryable table'?
- Export/Import SQLite-vec <-> git-blob-object
    - Use git-config to configure vector clustering/partitioning & size constraints
    - |
    ```bash
    # Store a text file containing vector data
    git hash-object -w vector_data.txt
    
    # Store a raw string/vector directly
    echo "[1.0, 2.0, 3.0]" | git hash-object -w --stdin
    ```
- JobSpy Package
    - Crawl based on given criteria
    - Extract matching jobs
    - Import to DB
- LinkedIn Package
- Playwright Script for programmatic Indeed Crawling+Interaction
- Cross-Hub Profile Syncing
    - Indeed Resume <-> LinkedIn Profile
- Personal Sites Syncing
    - Update Bullets based on my blog posts
    - Update Bullets based on my GitHub Contributions
    - Update Bullets based on my GitHub Projects
- Browser Extension 
    - Filling out Job Apps
    - Syncing Job app status to DB
    - Easily save/mark job for programmatic crawling/extraction later
    

What I did manually that could be scripted:
┌─────────────────────────────────┬───────────────────────────┬─────────────┬─────────────────────────────────┐
│              Step               │     Current (Manual)      │ Scriptable? │            Approach             │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Parse jobspy JSON output        │ jq commands               │ ✅ Yes      │ Node.js/Python script           │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Create/lookup organizations     │ SQL with INSERT OR IGNORE │ ✅ Yes      │ Upsert function                 │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Transform job data to schema    │ Manual field mapping      │ ✅ Yes      │ Transformation function         │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Generate content hash for dedup │ Not done                  │ ✅ Yes      │ Hash job description            │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Extract skills from description │ Manual reading            │ ✅ Yes      │ Keyword matching or LLM         │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Link skills to jobs             │ Manual SQL                │ ✅ Yes      │ Automated skill linker          │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Set interest level              │ Subjective judgment       │ ❌ No       │ Needs human or LLM              │
├─────────────────────────────────┼───────────────────────────┼─────────────┼─────────────────────────────────┤
│ Add notes/flags                 │ Subjective analysis       │ ⚠️ Partial  │ LLM can suggest, human confirms │
└─────────────────────────────────┴───────────────────────────┴─────────────┴─────────────────────────────────┘
Proposed architecture:

┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  jobspy MCP     │────▶│  job-importer.js │────▶│  SQLite DB      │
│  (search_jobs)  │     │  - transform     │     │  - jobs         │
└─────────────────┘     │  - deduplicate   │     │  - job_sources  │
                        │  - extract skills│     │  - job_skills   │
                        │  - upsert orgs   │     │  - organizations│
                        └──────────────────┘     └─────────────────┘

What must remain MCP/interactive:
- The jobspy search itself (MCP tool call)
- Final approval of which jobs to save
- Interest level and subjective assessments
- Resolving ambiguous skill matches

  What Can Be Scripted (Create job-importer.js)

  // Proposed script structure
  // .claude/mcp/jobspy/src/job-importer.js

  export function importJobs(jobspyResults, db) {
    for (const job of jobspyResults.jobs) {
      // 1. Upsert organization
      const orgId = upsertOrganization(db, job.company);

      // 2. Check for duplicates (by source+external_id)
      if (jobExists(db, job.site, job.id)) continue;

      // 3. Generate content hash for fuzzy dedup
      const contentHash = hashDescription(job.description);

      // 4. Transform and insert job
      const jobId = insertJob(db, { ...job, orgId, contentHash });

      // 5. Insert job source
      insertJobSource(db, jobId, job.site, job.id, job.jobUrl);

      // 6. Extract and link skills (keyword matching)
      const skills = extractSkills(job.description, knownSkills);
      linkJobSkills(db, jobId, skills);
    }
  }

Analysis Capabilities
- How do these jobs overlap? (whats common between them)
- How have the requirements for jobs like these changed over time?
- Group these jobs by level and category
- Search these jobs for red-flags
-
