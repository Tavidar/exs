// Business module — official Georgian reference data (registration & taxation).
//
// მხოლოდ ოფიციალური, შემოწმებადი ინფორმაცია. ზღვრები და განაკვეთები შეიძლება
// დროთა განმავლობაში შეიცვალოს — ყოველთვის უნდა გადამოწმდეს შემოსავლების
// სამსახურის (rs.ge) ოფიციალურ წყაროსთან. სისტემა არ იგონებს მოთხოვნებს.
//
// ეს ფაილი დეტერმინისტული ცოდნის ბაზაა: მუშაობს ოფლაინაც (AI-ს გარეშე).

/// მიკრო ბიზნესის წლიური ბრუნვის ზღვარი (₾).
pub const MICRO_TURNOVER_LIMIT_GEL: f64 = 30_000.0;
/// მცირე ბიზნესის წლიური ბრუნვის ზღვარი (₾).
pub const SMALL_TURNOVER_LIMIT_GEL: f64 = 500_000.0;
/// დღგ-ის სავალდებულო რეგისტრაციის ზღვარი 12 თვის ბრუნვაზე (₾).
pub const VAT_THRESHOLD_GEL: f64 = 100_000.0;

/// Tax classification outcome (deterministic, from official thresholds).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxClassification {
    /// machine code: "micro" | "small" | "standard"
    pub category: String,
    pub category_ka: String,
    pub rate_ka: String,
    pub vat_relevant: bool,
    pub reasoning_ka: String,
    pub obligations_ka: Vec<String>,
    /// 0.0..1.0 — confidence of the threshold-based decision.
    pub confidence: f64,
}

/// Classify a business by annual turnover (₾) and headcount, using official
/// Georgian thresholds. Employee count matters: micro status is for individual
/// entrepreneurs without hired staff.
pub fn classify_tax(annual_turnover_gel: f64, employees: i64) -> TaxClassification {
    let vat_relevant = annual_turnover_gel >= VAT_THRESHOLD_GEL;

    if annual_turnover_gel <= MICRO_TURNOVER_LIMIT_GEL && employees <= 0 {
        return TaxClassification {
            category: "micro".into(),
            category_ka: "მიკრო ბიზნესი".into(),
            rate_ka: "საშემოსავლო გადასახადი 0% (გარკვეული საქმიანობები გამორიცხულია).".into(),
            vat_relevant,
            reasoning_ka: format!(
                "მოსალოდნელი წლიური ბრუნვა (~{:.0} ₾) არ აღემატება მიკრო ბიზნესის ზღვარს ({:.0} ₾) და დაქირავებული თანამშრომელი არ არის.",
                annual_turnover_gel, MICRO_TURNOVER_LIMIT_GEL
            ),
            obligations_ka: vec![
                "მიკრო ბიზნესის სტატუსის მოპოვება შემოსავლების სამსახურში.".into(),
                "წლიური დეკლარაციის წარდგენა.".into(),
                "ნებადართული საქმიანობების ნუსხის დაცვა.".into(),
            ],
            confidence: 0.9,
        };
    }

    if annual_turnover_gel <= SMALL_TURNOVER_LIMIT_GEL {
        return TaxClassification {
            category: "small".into(),
            category_ka: "მცირე ბიზნესი".into(),
            rate_ka: "ბრუნვის გადასახადი 1% (ზღვრის გადაჭარბებისას ნაწილზე — 3%).".into(),
            vat_relevant,
            reasoning_ka: format!(
                "მოსალოდნელი წლიური ბრუნვა (~{:.0} ₾) ჯდება მცირე ბიზნესის ზღვარში ({:.0} ₾).{}",
                annual_turnover_gel,
                SMALL_TURNOVER_LIMIT_GEL,
                if vat_relevant {
                    format!(" ბრუნვა აღემატება დღგ-ის ზღვარს ({:.0} ₾) — დღგ-ზე რეგისტრაცია სავალდებულოა.", VAT_THRESHOLD_GEL)
                } else {
                    String::new()
                }
            ),
            obligations_ka: vec![
                "მცირე ბიზნესის სტატუსის მოპოვება და სალარო აპარატის გამოყენება.".into(),
                "ყოველთვიური დეკლარაცია 1% განაკვეთით.".into(),
                if vat_relevant {
                    "დღგ-ზე რეგისტრაცია და 18% დღგ-ის ადმინისტრირება.".into()
                } else {
                    "ბრუნვის მონიტორინგი დღგ-ის ზღვართან მიახლოებისას.".into()
                },
            ],
            confidence: 0.85,
        };
    }

    TaxClassification {
        category: "standard".into(),
        category_ka: "სტანდარტული რეჟიმი (შპს/იურიდიული პირი)".into(),
        rate_ka: "მოგების გადასახადი 15% (განაწილებისას), დივიდენდი 5%, დღგ 18%.".into(),
        vat_relevant: true,
        reasoning_ka: format!(
            "მოსალოდნელი წლიური ბრუნვა (~{:.0} ₾) აღემატება მცირე ბიზნესის ზღვარს ({:.0} ₾) — რეკომენდებულია იურიდიული პირი (შპს) სტანდარტული რეჟიმით.",
            annual_turnover_gel, SMALL_TURNOVER_LIMIT_GEL
        ),
        obligations_ka: vec![
            "შპს-ის რეგისტრაცია საჯარო რეესტრში.".into(),
            "დღგ-ზე რეგისტრაცია (18%).".into(),
            "ბუღალტრული აღრიცხვა და ყოველთვიური დეკლარაციები.".into(),
        ],
        confidence: 0.8,
    }
}

/// One step of the registration roadmap.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistrationStep {
    pub title_ka: String,
    pub detail_ka: String,
    pub institution_ka: String,
    pub cost_ka: String,
    pub duration_ka: String,
}

/// Recommended legal form + roadmap derived from the tax category.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistrationPlan {
    pub recommended_path_ka: String,
    pub institutions_ka: Vec<String>,
    pub required_documents_ka: Vec<String>,
    pub steps: Vec<RegistrationStep>,
    pub estimated_timeline_ka: String,
    pub estimated_cost_ka: String,
}

/// Build the registration roadmap. Individual entrepreneur for micro/small,
/// LLC (შპს) for the standard regime.
pub fn registration_plan(tax_category: &str) -> RegistrationPlan {
    let is_company = tax_category == "standard";

    let recommended_path_ka = if is_company {
        "შეზღუდული პასუხისმგებლობის საზოგადოება (შპს)".into()
    } else {
        "ინდივიდუალური მეწარმე (ფიზიკური პირი)".into()
    };

    let mut steps = vec![
        RegistrationStep {
            title_ka: "რეგისტრაცია იუსტიციის სახლში".into(),
            detail_ka: if is_company {
                "შპს-ის რეგისტრაცია: დამფუძნებლის პირადობა, წესდება, იურიდიული მისამართის დამადასტურებელი.".into()
            } else {
                "ინდ. მეწარმედ რეგისტრაცია პირადობის მოწმობით — ჩვეულებრივ იმავე დღეს.".into()
            },
            institution_ka: "იუსტიციის სახლი / საჯარო რეესტრის ეროვნული სააგენტო".into(),
            cost_ka: if is_company {
                "≈ 100 ₾ (სტანდარტული), ≈ 200 ₾ (დაჩქარებული)".into()
            } else {
                "უფასო ან მინიმალური მოსაკრებელი".into()
            },
            duration_ka: "1 სამუშაო დღე".into(),
        },
        RegistrationStep {
            title_ka: "საგადასახადო აღრიცხვა შემოსავლების სამსახურში".into(),
            detail_ka: "ელექტრონული პორტალის (rs.ge) აქტივაცია და საგადასახადო სტატუსის მოპოვება.".into(),
            institution_ka: "შემოსავლების სამსახური (rs.ge)".into(),
            cost_ka: "უფასო".into(),
            duration_ka: "1 დღე".into(),
        },
    ];

    if tax_category == "micro" {
        steps.push(RegistrationStep {
            title_ka: "მიკრო ბიზნესის სტატუსის მოპოვება".into(),
            detail_ka: "განცხადება მიკრო ბიზნესის სტატუსზე (საშემოსავლო 0%).".into(),
            institution_ka: "შემოსავლების სამსახური".into(),
            cost_ka: "უფასო".into(),
            duration_ka: "1–3 დღე".into(),
        });
    } else if tax_category == "small" {
        steps.push(RegistrationStep {
            title_ka: "მცირე ბიზნესის სტატუსისა და სალარო აპარატის გაფორმება".into(),
            detail_ka: "მცირე ბიზნესის სტატუსი (1%) და სალარო აპარატის რეგისტრაცია.".into(),
            institution_ka: "შემოსავლების სამსახური".into(),
            cost_ka: "სალაროს ღირებულება დამოკიდებულია მოდელზე".into(),
            duration_ka: "1–3 დღე".into(),
        });
    }

    let mut required_documents_ka = vec!["პირადობის მოწმობა".into()];
    if is_company {
        required_documents_ka.push("შპს-ის წესდება".into());
        required_documents_ka.push("იურიდიული მისამართის დამადასტურებელი".into());
        required_documents_ka.push("დამფუძნებელთა გადაწყვეტილება".into());
    }

    RegistrationPlan {
        recommended_path_ka,
        institutions_ka: vec![
            "იუსტიციის სახლი".into(),
            "საჯარო რეესტრის ეროვნული სააგენტო".into(),
            "შემოსავლების სამსახური (rs.ge)".into(),
        ],
        required_documents_ka,
        steps,
        estimated_timeline_ka: if is_company {
            "ჯამში ≈ 2–5 სამუშაო დღე".into()
        } else {
            "ჯამში ≈ 1–3 სამუშაო დღე".into()
        },
        estimated_cost_ka: if is_company {
            "≈ 100–250 ₾".into()
        } else {
            "≈ 0–50 ₾".into()
        },
    }
}

/// Extract the first monetary/numeric amount from free Georgian/Russian text
/// like "დაახლოებით 40 000 ₾" → 40000.0. Returns None if no digits.
pub fn parse_amount(text: &str) -> Option<f64> {
    let digits: String = text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<f64>().ok()
}

/// Extract the first whole number (e.g. employee count) from text.
pub fn parse_count(text: &str) -> Option<i64> {
    let first: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    first.parse::<i64>().ok()
}
