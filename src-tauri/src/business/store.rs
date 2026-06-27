// Business module — persistence over the shared SQLite connection.
//
// მუშაობს მხოლოდ business_* ცხრილებზე (იხ. 005_business_module.sql) — Core-ის
// items/events არასოდეს იცვლება აქედან. ყველა შეცდომა → BusinessError (Storage).

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::business::errors::BusinessError;
use crate::business::types::{DiscoveryAnswer, LaunchSession};

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn row_to_session(row: &Row<'_>) -> rusqlite::Result<LaunchSession> {
    Ok(LaunchSession {
        id: row.get("id")?,
        stage: row.get("stage")?,
        status: row.get("status")?,
        language: row.get("language")?,
        title: row.get("title")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const SESSION_COLS: &str =
    "id, stage, status, language, title, created_at, updated_at";

/// Create a new launch session (stage = discovery, status = active).
pub fn create_session(
    conn: &Connection,
    language: &str,
    title: Option<&str>,
) -> Result<LaunchSession, BusinessError> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO business_launch_session (id, language, title) VALUES (?1, ?2, ?3)",
        params![id, language, title],
    )?;
    get_session(conn, &id)?
        .ok_or_else(|| BusinessError::storage("session disappeared right after insert"))
}

/// Fetch one session by id.
pub fn get_session(conn: &Connection, id: &str) -> Result<Option<LaunchSession>, BusinessError> {
    let sql = format!("SELECT {SESSION_COLS} FROM business_launch_session WHERE id = ?1");
    let session = conn
        .query_row(&sql, params![id], row_to_session)
        .optional()?;
    Ok(session)
}

/// List sessions, newest first.
pub fn list_sessions(conn: &Connection) -> Result<Vec<LaunchSession>, BusinessError> {
    let sql = format!(
        "SELECT {SESSION_COLS} FROM business_launch_session ORDER BY created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_session)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Bump `updated_at` to now.
pub fn touch_session(conn: &Connection, id: &str) -> Result<(), BusinessError> {
    conn.execute(
        "UPDATE business_launch_session SET updated_at = ?2 WHERE id = ?1",
        params![id, now_iso()],
    )?;
    Ok(())
}

/// Advance/set the pipeline stage of a session.
pub fn set_stage(conn: &Connection, id: &str, stage: &str) -> Result<(), BusinessError> {
    conn.execute(
        "UPDATE business_launch_session SET stage = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, stage, now_iso()],
    )?;
    Ok(())
}

/// Insert or replace a discovery answer (one per question key per session).
pub fn upsert_answer(
    conn: &Connection,
    session_id: &str,
    question_key: &str,
    answer_text: Option<&str>,
    skipped: bool,
) -> Result<(), BusinessError> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO business_discovery_answer
            (id, session_id, question_key, answer_text, skipped, answered_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id, question_key) DO UPDATE SET
            answer_text = excluded.answer_text,
            skipped     = excluded.skipped,
            answered_at = excluded.answered_at",
        params![id, session_id, question_key, answer_text, skipped as i64, now_iso()],
    )?;
    Ok(())
}

/// All answers for a session, in answer order.
pub fn get_answers(conn: &Connection, session_id: &str) -> Result<Vec<DiscoveryAnswer>, BusinessError> {
    let mut stmt = conn.prepare(
        "SELECT question_key, answer_text, skipped, answered_at
         FROM business_discovery_answer
         WHERE session_id = ?1
         ORDER BY answered_at ASC",
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        let skipped: i64 = row.get("skipped")?;
        Ok(DiscoveryAnswer {
            question_key: row.get("question_key")?,
            answer_text: row.get("answer_text")?,
            skipped: skipped != 0,
            answered_at: row.get("answered_at")?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Persist a versioned business entity (BusinessProfile, TaxProfile, …).
/// Each call creates a new version; the latest version is the active one.
pub fn put_entity(
    conn: &Connection,
    session_id: &str,
    kind: &str,
    data_json: &str,
    confidence: Option<f64>,
) -> Result<i64, BusinessError> {
    let next_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM business_entity WHERE session_id = ?1 AND kind = ?2",
        params![session_id, kind],
        |r| r.get(0),
    )?;
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO business_entity
            (id, session_id, kind, version, schema_version, data_json, confidence)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)",
        params![id, session_id, kind, next_version, data_json, confidence],
    )?;
    Ok(next_version)
}

/// Latest stored JSON for a versioned entity kind, if any.
pub fn get_latest_entity(
    conn: &Connection,
    session_id: &str,
    kind: &str,
) -> Result<Option<String>, BusinessError> {
    let json = conn
        .query_row(
            "SELECT data_json FROM business_entity
             WHERE session_id = ?1 AND kind = ?2 ORDER BY version DESC LIMIT 1",
            params![session_id, kind],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(json)
}

/// Just the answered question keys (used to compute the next question).
pub fn answered_keys(conn: &Connection, session_id: &str) -> Result<Vec<String>, BusinessError> {
    let mut stmt = conn.prepare(
        "SELECT question_key FROM business_discovery_answer
         WHERE session_id = ?1 ORDER BY answered_at ASC",
    )?;
    let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>("question_key"))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
