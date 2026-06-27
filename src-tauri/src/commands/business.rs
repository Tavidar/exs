// Business commands — bridge between the UI and the ELS Business Gateway.
//
// ყველა ბრძანება აბრუნებს `Result<T, BusinessError>`-ს: წარუმატებლობა UI-ს
// მიდის სტრუქტურირებულად (code/message/retryable), არა ბრმა სტრიქონად.
// DB-ს ბლოკი იჭერება მხოლოდ მოკლედ; ELS არ ეხება Core-ის ბრძანებებს.

use std::sync::Arc;

use tauri::State;

use crate::ai::{self, types::AiProviderKind, types::AiRequest};
use crate::business::errors::BusinessError;
use crate::business::profiling::{self, BusinessProfileReport};
use crate::business::types::{DiscoveryQuestion, LaunchSession, SessionDetail};
use crate::business::{gateway, store};
use crate::db::Database;

fn lock_err(e: impl std::fmt::Display) -> BusinessError {
    BusinessError::internal(format!("database lock unavailable: {e}"))
}

/// Selected AI provider from local_config (defaults to mock — no key needed).
fn selected_provider(conn: &rusqlite::Connection) -> AiProviderKind {
    let s: Option<String> = conn
        .query_row("SELECT value FROM local_config WHERE key = 'ai_provider'", [], |r| r.get(0))
        .ok();
    AiProviderKind::from_str(s.as_deref().unwrap_or("mock"))
}

/// The full localized discovery interview (no DB access — pure content).
#[tauri::command]
pub fn business_discovery_questions() -> Vec<DiscoveryQuestion> {
    crate::business::discovery::all_questions()
}

/// Begin a new entrepreneur launch journey.
#[tauri::command]
pub fn business_start_session(
    db: State<'_, Database>,
    language: Option<String>,
    title: Option<String>,
) -> Result<SessionDetail, BusinessError> {
    let conn = db.conn.lock().map_err(lock_err)?;
    gateway::start_session(&conn, language.as_deref(), title.as_deref())
}

/// Full view of one launch session.
#[tauri::command]
pub fn business_get_session(
    db: State<'_, Database>,
    session_id: String,
) -> Result<SessionDetail, BusinessError> {
    let conn = db.conn.lock().map_err(lock_err)?;
    gateway::get_session_detail(&conn, &session_id)
}

/// All launch sessions, newest first.
#[tauri::command]
pub fn business_list_sessions(db: State<'_, Database>) -> Result<Vec<LaunchSession>, BusinessError> {
    let conn = db.conn.lock().map_err(lock_err)?;
    gateway::list_sessions(&conn)
}

/// Record (or skip) an answer to a discovery question.
#[tauri::command]
pub fn business_submit_answer(
    db: State<'_, Database>,
    session_id: String,
    question_key: String,
    answer_text: Option<String>,
    skipped: Option<bool>,
) -> Result<SessionDetail, BusinessError> {
    let conn = db.conn.lock().map_err(lock_err)?;
    gateway::submit_answer(
        &conn,
        &session_id,
        &question_key,
        answer_text.as_deref(),
        skipped.unwrap_or(false),
    )
}

/// Generate the Business Profile Report from the discovery answers.
///
/// Deterministic core (official Georgian tax/registration rules) always runs;
/// an AI-generated Georgian narrative is layered on via the AI Gateway and
/// gracefully falls back to the deterministic summary when AI is unavailable.
#[tauri::command]
pub async fn business_generate_profile(
    db: State<'_, Database>,
    session_id: String,
) -> Result<BusinessProfileReport, BusinessError> {
    let conn_arc = Arc::clone(&db.conn);

    // Pre-await: validate session, gather answers + selected provider, drop lock.
    let (answers, selected) = {
        let conn = conn_arc.lock().map_err(lock_err)?;
        if store::get_session(&conn, &session_id)?.is_none() {
            return Err(BusinessError::not_found(format!(
                "session '{session_id}' not found"
            )));
        }
        let answers = store::get_answers(&conn, &session_id)?;
        (answers, selected_provider(&conn))
    };

    // Deterministic draft from official rules — available even offline.
    let draft = profiling::build_draft(&session_id, &answers);

    // AI narrative through the gateway; graceful fallback to deterministic text.
    let prompt = profiling::narrative_prompt(&answers, &draft);
    let router = ai::build_router(selected);
    let req = AiRequest {
        query: prompt,
        language: "ka".to_string(),
        context_items: Vec::new(),
        context_files: Vec::new(),
    };
    let (analysis, generated_by) = match router.answer(&req).await {
        Ok(a) => (a.text, a.provider),
        Err(_) => (draft.deterministic_analysis.clone(), "deterministic".to_string()),
    };

    let report = draft.into_report(analysis, generated_by);

    // Post-await: persist the versioned BusinessProfile + advance the stage.
    {
        let conn = conn_arc.lock().map_err(lock_err)?;
        let json = serde_json::to_string(&report)
            .map_err(|e| BusinessError::internal(format!("serialize profile: {e}")))?;
        store::put_entity(&conn, &session_id, "BusinessProfile", &json, Some(report.confidence))?;
        store::set_stage(&conn, &session_id, "profiling")?;
    }

    Ok(report)
}
