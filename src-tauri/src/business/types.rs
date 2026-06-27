// Business module (ELS) — typed contract between the Rust engines and the
// frontend. snake_case სერიალიზაცია (Core-ის სტილი). ყველა სუბიექტი
// ვერსიონირდება DB-ში (იხ. 005_business_module.sql).

use serde::{Deserialize, Serialize};

/// ELS pipeline stage — discovery → … → completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchStage {
    Discovery,
    Profiling,
    Brand,
    Logo,
    Registration,
    Tax,
    Banking,
    Completed,
}

impl LaunchStage {
    pub fn as_str(self) -> &'static str {
        match self {
            LaunchStage::Discovery => "discovery",
            LaunchStage::Profiling => "profiling",
            LaunchStage::Brand => "brand",
            LaunchStage::Logo => "logo",
            LaunchStage::Registration => "registration",
            LaunchStage::Tax => "tax",
            LaunchStage::Banking => "banking",
            LaunchStage::Completed => "completed",
        }
    }
}

/// Versioned business entity kinds (DATA MODEL section of the spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    BusinessProfile,
    RegistrationProfile,
    TaxProfile,
    BrandProfile,
    LogoProfile,
    BankProfile,
    MarketProfile,
    GrowthProfile,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::BusinessProfile => "BusinessProfile",
            EntityKind::RegistrationProfile => "RegistrationProfile",
            EntityKind::TaxProfile => "TaxProfile",
            EntityKind::BrandProfile => "BrandProfile",
            EntityKind::LogoProfile => "LogoProfile",
            EntityKind::BankProfile => "BankProfile",
            EntityKind::MarketProfile => "MarketProfile",
            EntityKind::GrowthProfile => "GrowthProfile",
        }
    }
}

/// A launch session row (one entrepreneur journey).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchSession {
    pub id: String,
    pub stage: String,
    pub status: String,
    pub language: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// One discovery-interview question, fully localized (content lives in content.rs).
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryQuestion {
    pub key: String,
    /// 1-based position in the interview.
    pub index: usize,
    pub total: usize,
    /// კითხვა (the question itself).
    pub prompt: String,
    /// რატომ ვსვამთ ამ კითხვას.
    pub purpose: String,
    /// ახსნა — როგორ უნდა გავიგოთ კითხვა.
    pub explanation: String,
    /// მაგალითი პასუხი.
    pub example: String,
    pub optional: bool,
}

/// A persisted answer to a discovery question.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryAnswer {
    pub question_key: String,
    pub answer_text: Option<String>,
    pub skipped: bool,
    pub answered_at: String,
}

/// Progress snapshot returned after each answer / on request.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryProgress {
    pub session_id: String,
    pub stage: String,
    pub answered: usize,
    pub total: usize,
    pub complete: bool,
    /// The next unanswered question, or `None` when discovery is complete.
    pub next_question: Option<DiscoveryQuestion>,
}

/// Full session view: session row + collected answers + current progress.
#[derive(Debug, Clone, Serialize)]
pub struct SessionDetail {
    pub session: LaunchSession,
    pub answers: Vec<DiscoveryAnswer>,
    pub progress: DiscoveryProgress,
}
