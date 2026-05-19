use std::collections::HashMap;

use crate::types::{Finding, ReplacementGroup, SensitiveType};

pub fn normalize_original(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

pub fn build_groups(findings: &[Finding]) -> Vec<ReplacementGroup> {
    let mut sorted = findings.to_vec();
    sorted.sort_by_key(|finding| (finding.start, finding.end));

    let mut counters: HashMap<SensitiveType, usize> = HashMap::new();
    let mut groups: Vec<ReplacementGroup> = Vec::new();

    for finding in sorted {
        let normalized = normalize_original(&finding.text);
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.r#type == finding.r#type && group.normalized_original == normalized)
        {
            group.finding_ids.push(finding.id);
            continue;
        }

        let next = counters.entry(finding.r#type).or_default();
        *next += 1;
        groups.push(ReplacementGroup {
            id: format!("group-{}", groups.len() + 1),
            r#type: finding.r#type,
            original: finding.text,
            normalized_original: normalized,
            replacement: replacement_for(finding.r#type, *next),
            finding_ids: vec![finding.id],
            enabled: true,
        });
    }

    groups
}

fn replacement_for(r#type: SensitiveType, index: usize) -> String {
    let prefix = match r#type {
        SensitiveType::PersonName => "PERSON",
        SensitiveType::Organization => "ORG",
        SensitiveType::RoleOrPosition => "ROLE",
        SensitiveType::Location => "LOCATION",
        SensitiveType::Email => "EMAIL",
        SensitiveType::Phone => "PHONE",
        SensitiveType::Date => "DATE",
        SensitiveType::IdNumber => "ID",
        SensitiveType::Url => "URL",
        SensitiveType::OtherSensitive => "REDACTED",
    };
    format!("[{prefix}_{index}]")
}

#[cfg(test)]
mod tests {
    use crate::types::{FindingSource, SensitiveType};

    use super::*;

    #[test]
    fn groups_exact_normalized_repeats() {
        let findings = vec![
            finding("Jane  Doe", 0),
            finding("jane doe", 20),
            finding("Jane", 40),
        ];
        let groups = build_groups(&findings);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].finding_ids.len(), 2);
    }

    fn finding(text: &str, start: usize) -> Finding {
        Finding {
            id: format!("f-{start}"),
            r#type: SensitiveType::PersonName,
            start,
            end: start + text.len(),
            text: text.to_string(),
            source: FindingSource::Llm,
            confidence: Some(0.8),
            needs_review: Some(false),
        }
    }
}
