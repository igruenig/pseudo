use std::collections::HashMap;

use crate::types::{Finding, ReplacementGroup};

pub fn apply_replacements(text: &str, findings: &[Finding], groups: &[ReplacementGroup]) -> String {
    let by_id: HashMap<&str, &Finding> = findings.iter().map(|finding| (finding.id.as_str(), finding)).collect();
    let mut ranges = Vec::new();
    for group in groups.iter().filter(|group| group.enabled) {
        for finding_id in &group.finding_ids {
            if let Some(finding) = by_id.get(finding_id.as_str()) {
                ranges.push((*finding, group.replacement.as_str()));
            }
        }
    }
    ranges.sort_by_key(|(finding, _)| std::cmp::Reverse(finding.start));

    let mut output = text.to_string();
    for (finding, replacement) in ranges {
        if finding.end <= output.len() && finding.start <= finding.end {
            output.replace_range(finding.start..finding.end, replacement);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use crate::{
        grouping::build_groups,
        types::{Finding, FindingSource, SensitiveType},
    };

    use super::*;

    #[test]
    fn replaces_from_end_to_start() {
        let text = "Jane emailed jane@example.com.";
        let findings = vec![
            Finding {
                id: "f1".into(),
                r#type: SensitiveType::PersonName,
                start: 0,
                end: 4,
                text: "Jane".into(),
                source: FindingSource::Manual,
                confidence: Some(1.0),
                needs_review: Some(false),
            },
            Finding {
                id: "f2".into(),
                r#type: SensitiveType::Email,
                start: 13,
                end: 29,
                text: "jane@example.com".into(),
                source: FindingSource::Deterministic,
                confidence: Some(1.0),
                needs_review: Some(false),
            },
        ];
        let groups = build_groups(&findings);
        assert_eq!(apply_replacements(text, &findings, &groups), "[PERSON_1] emailed [EMAIL_1].");
    }
}
