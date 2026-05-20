use regex::Regex;
use uuid::Uuid;

use crate::types::{Finding, FindingSource, SensitiveType};

pub fn detect_deterministic(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    detect_regex(
        text,
        &mut findings,
        SensitiveType::Email,
        r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b",
    );
    detect_regex(
        text,
        &mut findings,
        SensitiveType::Url,
        r"\b(?:https?://|www\.)\S+",
    );
    findings
}

fn detect_regex(text: &str, findings: &mut Vec<Finding>, r#type: SensitiveType, pattern: &str) {
    let regex = Regex::new(pattern).expect("static regex compiles");
    for mat in regex.find_iter(text) {
        let matched = mat.as_str().trim_end_matches(['.', ',', ';', ')']);
        findings.push(Finding {
            id: Uuid::new_v4().to_string(),
            r#type,
            start: mat.start(),
            end: mat.start() + matched.len(),
            text: matched.to_string(),
            source: FindingSource::Deterministic,
            confidence: Some(1.0),
            needs_review: Some(false),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_email_and_url_only() {
        let findings =
            detect_deterministic("Email jane@example.com and see https://example.com/case.");
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.r#type == SensitiveType::Email));
        assert!(findings.iter().any(|f| f.r#type == SensitiveType::Url));
    }
}
