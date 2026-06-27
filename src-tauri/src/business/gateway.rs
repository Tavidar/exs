// Business Gateway — the typed isolation boundary (Core ↕ Gateway ↕ ELS).
//
// commands-ი მხოლოდ აქ შემოდის. Gateway აერთიანებს store-ს (persistence) და
// discovery-ს (engine logic), ყველაფერს ახვევს BusinessError-ში და იცავს
// იზოლაციას: ELS-ის ჩავარდნა ვერ ანგრევს Core-ს — სტრუქტურირებული შეცდომა
// ბრუნდება UI-სკენ. AI გამოძახებები (profiling, brand…) მომავალში მხოლოდ
// AI Gateway-ით განხორციელდება, არასოდეს პირდაპირ პროვაიდერთან.

use rusqlite::Connection;

use crate::business::errors::BusinessError;
use crate::business::types::{LaunchSession, SessionDetail};
use crate::business::{discovery, store};

/// Assemble the full session view (session + answers + discovery progress).
fn build_detail(conn: &Connection, session: LaunchSession) -> Result<SessionDetail, BusinessError> {
    let answers = store::get_answers(conn, &session.id)?;
    let keys = store::answered_keys(conn, &session.id)?;
    let progress = discovery::progress(&session.id, &session.stage, &keys);
    Ok(SessionDetail { session, answers, progress })
}

/// Load a session or fail with a structured NotFound.
fn require_session(conn: &Connection, session_id: &str) -> Result<LaunchSession, BusinessError> {
    store::get_session(conn, session_id)?
        .ok_or_else(|| BusinessError::not_found(format!("session '{session_id}' not found")))
}

/// Start a new entrepreneur launch journey (begins at the discovery stage).
pub fn start_session(
    conn: &Connection,
    language: Option<&str>,
    title: Option<&str>,
) -> Result<SessionDetail, BusinessError> {
    let lang = language.unwrap_or("ka");
    let session = store::create_session(conn, lang, title)?;
    build_detail(conn, session)
}

/// Full view of one session.
pub fn get_session_detail(conn: &Connection, session_id: &str) -> Result<SessionDetail, BusinessError> {
    let session = require_session(conn, session_id)?;
    build_detail(conn, session)
}

/// All sessions, newest first.
pub fn list_sessions(conn: &Connection) -> Result<Vec<LaunchSession>, BusinessError> {
    store::list_sessions(conn)
}

/// Record an answer to a discovery question and return the updated session view.
pub fn submit_answer(
    conn: &Connection,
    session_id: &str,
    question_key: &str,
    answer_text: Option<&str>,
    skipped: bool,
) -> Result<SessionDetail, BusinessError> {
    // Validate the session exists before touching answers (structured NotFound).
    let _ = require_session(conn, session_id)?;

    let question = discovery::question_by_key(question_key)
        .ok_or_else(|| BusinessError::invalid_input(format!("unknown question '{question_key}'")))?;

    let trimmed = answer_text.map(str::trim).filter(|s| !s.is_empty());

    if skipped {
        if !question.optional {
            return Err(BusinessError::invalid_input(
                "ამ კითხვის გამოტოვება არ შეიძლება — გთხოვთ უპასუხოთ.",
            ));
        }
    } else if trimmed.is_none() {
        return Err(BusinessError::invalid_input(
            "პასუხი ცარიელია — გთხოვთ შეავსოთ ან გამოტოვოთ (თუ დასაშვებია).",
        ));
    }

    store::upsert_answer(conn, session_id, question_key, trimmed, skipped)?;
    store::touch_session(conn, session_id)?;

    let refreshed = require_session(conn, session_id)?;
    build_detail(conn, refreshed)
}
