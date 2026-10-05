//! Persistence layer for AlignmentResult.
//!
//! For forge-62kb: rusqlite path only. The wa-sqlite path follows lu5s's
//! SkillStore pattern and lands in a successor bead (likely tied to
//! forge-5x2h's wa-sqlite consumer wiring).

#[cfg(not(target_arch = "wasm32"))]
use forge_core::ForgeError;

#[cfg(not(target_arch = "wasm32"))]
use super::result::AlignmentResult;

#[derive(Debug, Clone)]
pub struct AlignmentResultSummary {
    pub id: String,
    pub resume_id: String,
    pub jd_id: String,
    pub computed_at_ms: u64,
    pub overall_score: f64,
    pub gap_count: u32,
}

/// Insert an AlignmentResult. Caller supplies the row `id` (UUID).
/// `gap_count` is computed from `result.gap_report.entries.len()`.
#[cfg(not(target_arch = "wasm32"))]
pub fn insert_rusqlite(
    conn: &rusqlite::Connection,
    id: &str,
    result: &AlignmentResult,
) -> Result<(), ForgeError> {
    let json = serde_json::to_string(result)
        .map_err(|e| ForgeError::Internal(format!("serialize alignment result: {e}")))?;
    let gap_count = result.gap_report.entries.len() as i64;
    conn.execute(
        "INSERT INTO alignment_results
         (id, resume_id, jd_id, computed_at, overall_score, gap_count, result_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            id,
            result.resume_id,
            result.jd_id,
            result.computed_at_ms as i64,
            result.overall_score,
            gap_count,
            json,
        ],
    )
    .map_err(|e| ForgeError::Internal(format!("insert alignment_results: {e}")))?;
    Ok(())
}

/// Read the most-recent AlignmentResult for a (resume_id, jd_id) pair, or
/// None if no row exists.
#[cfg(not(target_arch = "wasm32"))]
pub fn get_latest_rusqlite(
    conn: &rusqlite::Connection,
    resume_id: &str,
    jd_id: &str,
) -> Result<Option<AlignmentResult>, ForgeError> {
    let mut stmt = conn
        .prepare(
            "SELECT result_json FROM alignment_results
             WHERE resume_id = ?1 AND jd_id = ?2
             ORDER BY computed_at DESC
             LIMIT 1",
        )
        .map_err(|e| ForgeError::Internal(format!("prepare get_latest: {e}")))?;
    let mut rows = stmt
        .query(rusqlite::params![resume_id, jd_id])
        .map_err(|e| ForgeError::Internal(format!("query get_latest: {e}")))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| ForgeError::Internal(format!("row: {e}")))?
    {
        let json: String = row
            .get(0)
            .map_err(|e| ForgeError::Internal(format!("col: {e}")))?;
        let result = serde_json::from_str(&json)
            .map_err(|e| ForgeError::Internal(format!("deserialize: {e}")))?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// List summaries (NOT full AlignmentResults) for a resume, most-recent first.
#[cfg(not(target_arch = "wasm32"))]
pub fn list_for_resume_rusqlite(
    conn: &rusqlite::Connection,
    resume_id: &str,
    limit: i64,
) -> Result<Vec<AlignmentResultSummary>, ForgeError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, resume_id, jd_id, computed_at, overall_score, gap_count
             FROM alignment_results
             WHERE resume_id = ?1
             ORDER BY computed_at DESC
             LIMIT ?2",
        )
        .map_err(|e| ForgeError::Internal(format!("prepare list_for_resume: {e}")))?;
    let mut rows = stmt
        .query(rusqlite::params![resume_id, limit])
        .map_err(|e| ForgeError::Internal(format!("query list_for_resume: {e}")))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| ForgeError::Internal(format!("row: {e}")))?
    {
        let id: String = row
            .get(0)
            .map_err(|e| ForgeError::Internal(format!("col 0: {e}")))?;
        let resume_id: String = row
            .get(1)
            .map_err(|e| ForgeError::Internal(format!("col 1: {e}")))?;
        let jd_id: String = row
            .get(2)
            .map_err(|e| ForgeError::Internal(format!("col 2: {e}")))?;
        let computed_at: i64 = row
            .get(3)
            .map_err(|e| ForgeError::Internal(format!("col 3: {e}")))?;
        let overall_score: f64 = row
            .get(4)
            .map_err(|e| ForgeError::Internal(format!("col 4: {e}")))?;
        let gap_count: i64 = row
            .get(5)
            .map_err(|e| ForgeError::Internal(format!("col 5: {e}")))?;
        out.push(AlignmentResultSummary {
            id,
            resume_id,
            jd_id,
            computed_at_ms: computed_at as u64,
            overall_score,
            gap_count: gap_count as u32,
        });
    }
    Ok(out)
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::alignment::result::*;

    /// Migration 054 SQL inlined directly so the test doesn't need the
    /// MIGRATIONS slice (which lives in forge-sdk on this branch and isn't
    /// pub). When forge-lu5s lands on main, this can switch to
    /// `forge_core::migrations::MIGRATIONS` lookup.
    const MIGRATION_054: &str =
        include_str!("../../../../packages/core/src/db/migrations/054_alignment_results.sql");

    fn open_in_memory() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(MIGRATION_054)
            .expect("apply migration 054");
        conn
    }

    fn sample_result() -> AlignmentResult {
        AlignmentResult {
            resume_id: "r1".into(),
            jd_id: "jd1".into(),
            computed_at_ms: 1_700_000_000_000,
            overall_score: 0.78,
            per_skill_scores: vec![],
            gap_report: GapReport {
                entries: vec![GapEntry {
                    required_skill_id: "x".into(),
                    severity: 0.4,
                    best_match: None,
                    required_level: None,
                }],
            },
            strength_report: StrengthReport::default(),
            coverage_report: CoverageReport {
                strong: 0,
                moderate: 0,
                weak: 0,
                gap: 1,
                total_required: 1,
                coverage_pct: 0.0,
            },
            provenance: vec![],
        }
    }

    #[test]
    fn insert_then_get_latest_roundtrips() {
        let conn = open_in_memory();
        let result = sample_result();
        insert_rusqlite(&conn, "row-1", &result).unwrap();
        let back = get_latest_rusqlite(&conn, "r1", "jd1").unwrap().unwrap();
        assert_eq!(back.overall_score, 0.78);
        assert_eq!(back.gap_report.entries.len(), 1);
    }

    #[test]
    fn get_latest_picks_most_recent_row() {
        let conn = open_in_memory();
        let mut older = sample_result();
        older.overall_score = 0.50;
        older.computed_at_ms = 1_000_000_000_000;
        insert_rusqlite(&conn, "older", &older).unwrap();
        let mut newer = sample_result();
        newer.overall_score = 0.85;
        newer.computed_at_ms = 2_000_000_000_000;
        insert_rusqlite(&conn, "newer", &newer).unwrap();
        let back = get_latest_rusqlite(&conn, "r1", "jd1").unwrap().unwrap();
        assert!((back.overall_score - 0.85).abs() < 1e-6);
    }

    #[test]
    fn get_latest_returns_none_when_no_rows() {
        let conn = open_in_memory();
        let back = get_latest_rusqlite(&conn, "r-none", "jd-none").unwrap();
        assert!(back.is_none());
    }

    #[test]
    fn list_for_resume_returns_summaries_descending_by_computed_at() {
        let conn = open_in_memory();
        let mut older = sample_result();
        older.overall_score = 0.5;
        older.computed_at_ms = 1_000_000_000_000;
        insert_rusqlite(&conn, "older", &older).unwrap();
        let mut newer = sample_result();
        newer.overall_score = 0.85;
        newer.computed_at_ms = 2_000_000_000_000;
        insert_rusqlite(&conn, "newer", &newer).unwrap();
        let summaries = list_for_resume_rusqlite(&conn, "r1", 10).unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "newer", "most-recent first");
        assert_eq!(summaries[1].id, "older");
        assert_eq!(summaries[0].gap_count, 1);
    }

    #[test]
    fn list_for_resume_respects_limit() {
        let conn = open_in_memory();
        for i in 0..5 {
            let mut r = sample_result();
            r.computed_at_ms = (i + 1) as u64 * 1_000_000_000_000;
            insert_rusqlite(&conn, &format!("row-{i}"), &r).unwrap();
        }
        let summaries = list_for_resume_rusqlite(&conn, "r1", 3).unwrap();
        assert_eq!(summaries.len(), 3);
    }
}
