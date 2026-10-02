-- Forge Resume Builder — Initial Schema
-- Migration: 001_initial
-- Date: 2026-03-28
--
-- NOTE: PRAGMAs (journal_mode, foreign_keys) are set by the connection
-- helper BEFORE migrations run, NOT inside migration files.
-- SQLite ignores PRAGMA foreign_keys inside transactions.

-- Employers
CREATE TABLE employers (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Projects
CREATE TABLE projects (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  employer_id TEXT REFERENCES employers(id) ON DELETE SET NULL,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_projects_employer ON projects(employer_id);

-- Sources
CREATE TABLE sources (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  employer_id TEXT REFERENCES employers(id) ON DELETE SET NULL,
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  start_date TEXT,
  end_date TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'approved', 'deriving')),
  updated_by TEXT NOT NULL DEFAULT 'human' CHECK(updated_by IN ('human', 'ai')),
  last_derived_at TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_sources_status ON sources(status);
CREATE INDEX idx_sources_employer ON sources(employer_id);
CREATE INDEX idx_sources_project ON sources(project_id);

-- Prompt Log (normalized prompt storage)
CREATE TABLE prompt_logs (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  entity_type TEXT NOT NULL CHECK(entity_type IN ('bullet', 'perspective')),
  entity_id TEXT NOT NULL,
  prompt_template TEXT NOT NULL,
  prompt_input TEXT NOT NULL,
  raw_response TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_prompt_logs_entity ON prompt_logs(entity_type, entity_id);

-- Bullets
CREATE TABLE bullets (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE RESTRICT,
  content TEXT NOT NULL,
  source_content_snapshot TEXT NOT NULL,
  metrics TEXT,
  status TEXT NOT NULL DEFAULT 'pending_review' CHECK(status IN ('draft', 'pending_review', 'approved', 'rejected')),
  rejection_reason TEXT,
  prompt_log_id TEXT REFERENCES prompt_logs(id) ON DELETE SET NULL,
  approved_at TEXT,
  approved_by TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_bullets_source ON bullets(source_id);
CREATE INDEX idx_bullets_status ON bullets(status);

-- Bullet Technologies (junction table)
CREATE TABLE bullet_technologies (
  bullet_id TEXT NOT NULL REFERENCES bullets(id) ON DELETE CASCADE,
  technology TEXT NOT NULL,
  PRIMARY KEY (bullet_id, technology)
) STRICT;

CREATE INDEX idx_bullet_tech_technology ON bullet_technologies(technology);

-- Perspectives
CREATE TABLE perspectives (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  bullet_id TEXT NOT NULL REFERENCES bullets(id) ON DELETE RESTRICT,
  content TEXT NOT NULL,
  bullet_content_snapshot TEXT NOT NULL,
  target_archetype TEXT,
  domain TEXT,
  framing TEXT NOT NULL CHECK(framing IN ('accomplishment', 'responsibility', 'context')),
  status TEXT NOT NULL DEFAULT 'pending_review' CHECK(status IN ('draft', 'pending_review', 'approved', 'rejected')),
  rejection_reason TEXT,
  prompt_log_id TEXT REFERENCES prompt_logs(id) ON DELETE SET NULL,
  approved_at TEXT,
  approved_by TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX idx_perspectives_bullet ON perspectives(bullet_id);
CREATE INDEX idx_perspectives_status ON perspectives(status);
CREATE INDEX idx_perspectives_archetype ON perspectives(target_archetype);
CREATE INDEX idx_perspectives_domain ON perspectives(domain);

-- Skills
CREATE TABLE skills (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL UNIQUE,
  category TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Bullet Skills (junction)
CREATE TABLE bullet_skills (
  bullet_id TEXT NOT NULL REFERENCES bullets(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  PRIMARY KEY (bullet_id, skill_id)
) STRICT;

-- Perspective Skills (junction)
CREATE TABLE perspective_skills (
  perspective_id TEXT NOT NULL REFERENCES perspectives(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  PRIMARY KEY (perspective_id, skill_id)
) STRICT;

-- Resumes
CREATE TABLE resumes (
  id TEXT PRIMARY KEY CHECK(typeof(id) = 'text' AND length(id) = 36),
  name TEXT NOT NULL,
  target_role TEXT NOT NULL,
  target_employer TEXT NOT NULL,
  archetype TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'final')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

-- Resume Perspectives (join table)
CREATE TABLE resume_perspectives (
  resume_id TEXT NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
  perspective_id TEXT NOT NULL REFERENCES perspectives(id) ON DELETE RESTRICT,
  section TEXT NOT NULL CHECK(section IN ('summary', 'work_history', 'projects', 'education', 'skills', 'awards')),
  position INTEGER NOT NULL,
  PRIMARY KEY (resume_id, perspective_id)
) STRICT;

CREATE INDEX idx_resume_perspectives_resume ON resume_perspectives(resume_id, section, position);

-- Migrations tracking
-- NOTE: _migrations intentionally does NOT use STRICT mode.
-- INTEGER PRIMARY KEY AUTOINCREMENT requires non-STRICT for compatibility.
CREATE TABLE _migrations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO _migrations (name) VALUES ('001_initial');
