// Business module (ELS) — structured, isolated errors.
//
// მოდულის ნებისმიერი შეცდომა ბრუნდება სტრუქტურირებულად (კოდი + ტექსტი +
// retryable) და არასოდეს ანგრევს Core-ს, მონაცემთა ბაზის შრეს ან UI-ს.
// commands-ი აბრუნებს `Result<T, BusinessError>`-ს — Err სერიალიზდება
// ფრონტისთვის როგორც ობიექტი, არა ბრმა სტრიქონი.

use serde::Serialize;

/// Machine-readable error category for the frontend.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusinessErrorCode {
    /// მოთხოვნილი სესია/სუბიექტი ვერ მოიძებნა.
    NotFound,
    /// არასწორი ან არასრული შემავალი მონაცემები.
    InvalidInput,
    /// მონაცემთა ბაზის შრის შეცდომა (persistence).
    Storage,
    /// Core-თან/AI Gateway-სთან კავშირის შეცდომა Business Gateway-ის საზღვარზე.
    Gateway,
    /// გაუთვალისწინებელი შიდა შეცდომა.
    Internal,
}

/// Structured failure returned across the Tauri command boundary.
#[derive(Debug, Clone, Serialize)]
pub struct BusinessError {
    pub code: BusinessErrorCode,
    pub message: String,
    /// თუ `true`, UI-ს შეუძლია უსაფრთხოდ გაიმეოროს ოპერაცია (retry).
    pub retryable: bool,
}

impl BusinessError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { code: BusinessErrorCode::NotFound, message: message.into(), retryable: false }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self { code: BusinessErrorCode::InvalidInput, message: message.into(), retryable: false }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self { code: BusinessErrorCode::Storage, message: message.into(), retryable: true }
    }

    pub fn gateway(message: impl Into<String>) -> Self {
        Self { code: BusinessErrorCode::Gateway, message: message.into(), retryable: true }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self { code: BusinessErrorCode::Internal, message: message.into(), retryable: false }
    }
}

impl std::fmt::Display for BusinessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for BusinessError {}

/// rusqlite errors are funnelled into a single structured Storage error so a DB
/// hiccup never leaks raw SQL state to the UI or aborts the module.
impl From<rusqlite::Error> for BusinessError {
    fn from(e: rusqlite::Error) -> Self {
        BusinessError::storage(e.to_string())
    }
}
