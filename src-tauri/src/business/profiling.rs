// Business Profiling Engine — turns discovery answers into a Business Profile
// Report (spec RESPONSE FORMAT). Deterministic core (official thresholds, works
// offline); an optional AI-generated Georgian narrative is layered on top by the
// command via the AI Gateway (never a hardcoded provider here).

use std::collections::HashMap;

use serde::Serialize;

use crate::business::knowledge::{self, RegistrationPlan, TaxClassification};
use crate::business::types::DiscoveryAnswer;

/// Full Business Profile Report returned to the UI and persisted (versioned).
#[derive(Debug, Clone, Serialize)]
pub struct BusinessProfileReport {
    pub session_id: String,
    pub business_type_ka: String,
    pub industry_ka: String,
    pub scale_ka: String,
    pub risk_level_ka: String,
    pub growth_potential_ka: String,
    pub tax: TaxClassification,
    pub registration: RegistrationPlan,
    pub banking_requirements_ka: Vec<String>,
    pub marketing_requirements_ka: Vec<String>,
    // ── spec RESPONSE FORMAT ──
    pub analysis_ka: String,
    pub recommendations_ka: Vec<String>,
    pub confidence: f64,
    pub risks_ka: Vec<String>,
    pub next_actions_ka: Vec<String>,
    pub required_documents_ka: Vec<String>,
    pub estimated_timeline_ka: String,
    pub estimated_cost_ka: String,
    /// Provider name ("openai"/"gemini"/"claude") or "deterministic" when AI was unavailable.
    pub generated_by: String,
}

/// Deterministic intermediate built purely from the answers + official rules.
pub struct ProfileDraft {
    pub session_id: String,
    pub turnover: f64,
    pub employees: i64,
    pub online: bool,
    pub has_physical: bool,
    pub tax: TaxClassification,
    pub registration: RegistrationPlan,
    pub business_type_ka: String,
    pub industry_ka: String,
    pub scale_ka: String,
    pub risk_level_ka: String,
    pub growth_potential_ka: String,
    pub deterministic_analysis: String,
}

fn answers_map(answers: &[DiscoveryAnswer]) -> HashMap<&str, &str> {
    answers
        .iter()
        .filter_map(|a| {
            a.answer_text
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| (a.question_key.as_str(), t))
        })
        .collect()
}

/// Build the deterministic draft (no AI). Always succeeds — missing answers fall
/// back to safe defaults so the engine degrades gracefully (offline support).
pub fn build_draft(session_id: &str, answers: &[DiscoveryAnswer]) -> ProfileDraft {
    let map = answers_map(answers);

    let turnover = map
        .get("expected_turnover")
        .and_then(|t| knowledge::parse_amount(t))
        .unwrap_or(0.0);
    let employees = map
        .get("number_of_employees")
        .and_then(|t| knowledge::parse_count(t))
        .unwrap_or(0);

    let has_product = map.contains_key("product_category");
    let has_services = map.contains_key("services_category");
    let online = map.contains_key("online_presence");
    let has_physical = map.contains_key("physical_location");
    let import_export = map.contains_key("import_export");
    let has_growth = map.contains_key("growth_plans");

    let tax = knowledge::classify_tax(turnover, employees);
    let registration = knowledge::registration_plan(&tax.category);

    let business_type_ka = match (has_product, has_services) {
        (true, true) => "პროდუქტი და მომსახურება".to_string(),
        (true, false) => "პროდუქტის გაყიდვა".to_string(),
        (false, true) => "მომსახურება".to_string(),
        (false, false) => "შერეული / დასაზუსტებელი".to_string(),
    };

    let industry_ka = map
        .get("product_category")
        .or_else(|| map.get("services_category"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "ზოგადი".to_string());

    let scale_ka = match tax.category.as_str() {
        "micro" => "მიკრო / სტარტაპი".to_string(),
        "small" => "მცირე ბიზნესი".to_string(),
        _ => "საშუალო ან მსხვილი".to_string(),
    };

    let risk_level_ka = if import_export {
        "საშუალო-მაღალი (იმპორტი/ექსპორტი, საბაჟო)".to_string()
    } else if has_physical {
        "საშუალო (ფიზიკური სივრცე, ქირა, ნებართვები)".to_string()
    } else {
        "დაბალი (მცირე საწყისი დანახარჯები)".to_string()
    };

    let growth_potential_ka = if has_growth {
        "მაღალი — მკაფიო ზრდის გეგმა".to_string()
    } else {
        "საშუალო — ზრდის გეგმა დასაზუსტებელია".to_string()
    };

    let idea = map.get("business_idea").copied().unwrap_or("(იდეა დასაზუსტებელია)");
    let deterministic_analysis = format!(
        "იდეა: {idea}. ტიპი: {business_type_ka}, მასштაბი: {scale_ka}. \
         მოსალოდნელი ბრუნვა ~{:.0} ₾, თანამშრომლები: {employees}. \
         საგადასახადო რეჟიმი: {}. რეკომენდებული ფორმა: {}.",
        turnover,
        tax.category_ka,
        registration.recommended_path_ka,
    );

    ProfileDraft {
        session_id: session_id.to_string(),
        turnover,
        employees,
        online,
        has_physical,
        tax,
        registration,
        business_type_ka,
        industry_ka,
        scale_ka,
        risk_level_ka,
        growth_potential_ka,
        deterministic_analysis,
    }
}

impl ProfileDraft {
    fn banking_requirements(&self) -> Vec<String> {
        let mut out = vec!["ბიზნეს ანგარიში (ფიზიკურისგან გამიჯნული)".to_string()];
        if self.online {
            out.push("ონლაინ-ეკვაირინგი გადახდებისთვის".to_string());
        }
        if self.has_physical {
            out.push("POS ტერმინალი ადგილზე გადახდისთვის".to_string());
        }
        if self.tax.vat_relevant {
            out.push("დღგ-ის ადმინისტრირებაზე მორგებული ბუღალტერია".to_string());
        }
        out
    }

    fn marketing_requirements(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.online {
            out.push("სოციალური ქსელები (Instagram/Facebook) და გვერდი".to_string());
            out.push("ბრენდინგი: სახელი, ლოგო, დომენი".to_string());
        } else {
            out.push("ლოკალური მარკეტინგი და რეკომენდაციები".to_string());
        }
        out
    }

    fn recommendations(&self) -> Vec<String> {
        let mut out = vec![
            format!("აირჩიეთ ფორმა: {}", self.registration.recommended_path_ka),
            format!("საგადასახადო რეჟიმი: {}", self.tax.category_ka),
        ];
        if self.tax.vat_relevant {
            out.push("დაგეგმეთ დღგ-ზე რეგისტრაცია წინასწარ.".to_string());
        }
        out.push("გახსენით ცალკე ბიზნეს-ანგარიში პირადი ფინანსებისგან გასამიჯნად.".to_string());
        out
    }

    fn risks(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.turnover <= 0.0 {
            out.push("ბრუნვა მითითებული არ არის — საგადასახადო შეფასება სავარაუდოა.".to_string());
        }
        if self.tax.vat_relevant {
            out.push("დღგ-ის ზღვრის გადაჭარბება ზრდის ადმინისტრირების ტვირთს.".to_string());
        }
        out.push("ზღვრები/განაკვეთები გადაამოწმეთ rs.ge-ზე — კანონმდებლობა იცვლება.".to_string());
        out
    }

    fn next_actions(&self) -> Vec<String> {
        vec![
            "დაასრულეთ აღმოჩენის ინტერვიუ ყველა კითხვაზე.".to_string(),
            format!("დაიწყეთ რეგისტრაცია: {}", self.registration.recommended_path_ka),
            "გადადით ბრენდის შექმნის ეტაპზე.".to_string(),
        ]
    }

    /// Assemble the final report, attaching the (AI or deterministic) narrative.
    pub fn into_report(self, analysis_ka: String, generated_by: String) -> BusinessProfileReport {
        let confidence = self.tax.confidence;
        let banking_requirements_ka = self.banking_requirements();
        let marketing_requirements_ka = self.marketing_requirements();
        let recommendations_ka = self.recommendations();
        let risks_ka = self.risks();
        let next_actions_ka = self.next_actions();
        let required_documents_ka = self.registration.required_documents_ka.clone();
        let estimated_timeline_ka = self.registration.estimated_timeline_ka.clone();
        let estimated_cost_ka = self.registration.estimated_cost_ka.clone();

        BusinessProfileReport {
            session_id: self.session_id,
            business_type_ka: self.business_type_ka,
            industry_ka: self.industry_ka,
            scale_ka: self.scale_ka,
            risk_level_ka: self.risk_level_ka,
            growth_potential_ka: self.growth_potential_ka,
            tax: self.tax,
            registration: self.registration,
            banking_requirements_ka,
            marketing_requirements_ka,
            analysis_ka,
            recommendations_ka,
            confidence,
            risks_ka,
            next_actions_ka,
            required_documents_ka,
            estimated_timeline_ka,
            estimated_cost_ka,
            generated_by,
        }
    }
}

/// Georgian prompt for the AI Gateway. Grounds the model on the collected answers
/// and the deterministic classification; asks for a short, natural Georgian analysis.
pub fn narrative_prompt(answers: &[DiscoveryAnswer], draft: &ProfileDraft) -> String {
    let map = answers_map(answers);
    let mut lines = String::new();
    for (k, v) in &map {
        lines.push_str(&format!("- {k}: {v}\n"));
    }
    format!(
        "შენ ხარ ქართველი მეწარმის თანადამფუძნებელი-კონსულტანტი. ქვემოთ მოცემულია \
         ბიზნესის აღმოჩენის ინტერვიუს პასუხები:\n{lines}\n\
         დეტერმინისტული შეფასება: საგადასახადო რეჟიმი — {}, რეკომენდებული ფორმა — {}, \
         მასშტაბი — {}.\n\n\
         დაწერე მოკლე (3–5 წინადადება), ბუნებრივი ქართული ანალიზი ამ ბიზნესის \
         სიცოცხლისუნარიანობასა და მთავარ ნაბიჯებზე. ნუ გამოიგონებ ფაქტებს; დაეყრდენი \
         მხოლოდ მოცემულ ინფორმაციას.",
        draft.tax.category_ka, draft.registration.recommended_path_ka, draft.scale_ka,
    )
}
