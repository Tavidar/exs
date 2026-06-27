// Business module — Entrepreneur Launch System (ELS).
//
// იზოლირებული ბიზნეს-მოდული. არ ცვლის Core-ის სერვისებს პირდაპირ; Core-თან
// ურთიერთობა მხოლოდ Business Gateway-ით, ტიპიზებული ინტერფეისებით. ჩავარდნა
// იზოლირებულია — ბრუნდება სტრუქტურირებული BusinessError.
//
// არქიტექტურა:  content (ka) → discovery (engine) → store (persistence)
//               ↘ gateway (boundary) ↗  →  commands::business (Tauri)

pub mod content;
pub mod discovery;
pub mod errors;
pub mod gateway;
pub mod knowledge;
pub mod profiling;
pub mod store;
pub mod types;
