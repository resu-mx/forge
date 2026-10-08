-- Migration 055: dataset_meta
--
-- Key-value table that marks a dataset as generated (e.g. kind = 'generated').
-- Written ONLY by the demo-data generator (packages/demo-data); it stays EMPTY
-- in real user databases, so an empty table means "not a generated dataset".
--
-- Uses only STRICT-compatible types so the same SQL runs on the TS server,
-- the Rust server, and the in-browser wasm build.

CREATE TABLE IF NOT EXISTS dataset_meta (
  key TEXT PRIMARY KEY CHECK(length(key) BETWEEN 1 AND 64),
  value TEXT NOT NULL
) STRICT;
