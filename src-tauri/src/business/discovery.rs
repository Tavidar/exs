// Business Discovery Engine — sequential interview logic.
//
// აშენებს ლოკალიზებულ კითხვებს content-იდან, აბრუნებს შემდეგ დაუსმელ
// კითხვას და ითვლის ინტერვიუს პროგრესს. წმინდა ლოგიკა — DB-ს არ ეხება.

use crate::business::content::{self, QuestionSpec};
use crate::business::types::{DiscoveryProgress, DiscoveryQuestion};

fn to_question(spec: &QuestionSpec, idx: usize, total: usize) -> DiscoveryQuestion {
    DiscoveryQuestion {
        key: spec.key.to_string(),
        index: idx + 1,
        total,
        prompt: spec.prompt.to_string(),
        purpose: spec.purpose.to_string(),
        explanation: spec.explanation.to_string(),
        example: spec.example.to_string(),
        optional: spec.optional,
    }
}

/// The full ordered interview, localized.
pub fn all_questions() -> Vec<DiscoveryQuestion> {
    let total = content::total();
    content::DISCOVERY_QUESTIONS
        .iter()
        .enumerate()
        .map(|(i, spec)| to_question(spec, i, total))
        .collect()
}

/// Look up a single question by its key (validates incoming answers).
pub fn question_by_key(key: &str) -> Option<DiscoveryQuestion> {
    let total = content::total();
    content::DISCOVERY_QUESTIONS
        .iter()
        .position(|q| q.key == key)
        .map(|i| to_question(&content::DISCOVERY_QUESTIONS[i], i, total))
}

/// First question (in order) whose key is not yet in `answered_keys`.
pub fn next_question(answered_keys: &[String]) -> Option<DiscoveryQuestion> {
    let total = content::total();
    content::DISCOVERY_QUESTIONS
        .iter()
        .enumerate()
        .find(|(_, spec)| !answered_keys.iter().any(|k| k == spec.key))
        .map(|(i, spec)| to_question(spec, i, total))
}

/// Build a progress snapshot from the set of already-answered question keys.
pub fn progress(session_id: &str, stage: &str, answered_keys: &[String]) -> DiscoveryProgress {
    let total = content::total();
    let next = next_question(answered_keys);
    DiscoveryProgress {
        session_id: session_id.to_string(),
        stage: stage.to_string(),
        answered: answered_keys.len().min(total),
        total,
        complete: next.is_none(),
        next_question: next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_question_is_business_idea() {
        let q = next_question(&[]).expect("must have a first question");
        assert_eq!(q.key, "business_idea");
        assert_eq!(q.index, 1);
    }

    #[test]
    fn next_skips_answered_in_order() {
        let answered = vec!["business_idea".to_string()];
        let q = next_question(&answered).expect("must have a next question");
        assert_eq!(q.key, "target_audience");
    }

    #[test]
    fn progress_is_complete_when_all_answered() {
        let answered: Vec<String> = content::DISCOVERY_QUESTIONS
            .iter()
            .map(|q| q.key.to_string())
            .collect();
        let p = progress("s1", "discovery", &answered);
        assert!(p.complete);
        assert!(p.next_question.is_none());
        assert_eq!(p.answered, p.total);
    }

    #[test]
    fn unknown_key_has_no_question() {
        assert!(question_by_key("does_not_exist").is_none());
    }
}
