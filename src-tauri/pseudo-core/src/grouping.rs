use std::collections::HashMap;

use crate::types::{Finding, ReplacementGroup, SensitiveType};

pub fn normalize_original(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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
            .find(|group| should_group(group, finding.r#type, &normalized))
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

fn should_group(group: &ReplacementGroup, r#type: SensitiveType, normalized: &str) -> bool {
    if group.r#type != r#type {
        return false;
    }
    if group.normalized_original == normalized {
        return true;
    }
    if r#type != SensitiveType::PersonName {
        return false;
    }

    let group_alias = person_alias_key(&group.normalized_original);
    let finding_alias = person_alias_key(normalized);
    if group_alias.is_empty() || group_alias != finding_alias {
        return false;
    }
    let group_simple = strip_trailing_s_tokens(&group.normalized_original);
    let finding_simple = strip_trailing_s_tokens(normalized);
    group_simple.contains(&finding_simple) || finding_simple.contains(&group_simple)
}

fn person_alias_key(value: &str) -> String {
    value
        .split_whitespace()
        .last()
        .unwrap_or(value)
        .trim_end_matches('s')
        .to_string()
}

fn strip_trailing_s_tokens(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| token.trim_end_matches('s'))
        .collect::<Vec<_>>()
        .join(" ")
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

    #[test]
    fn groups_person_surname_aliases() {
        let findings = vec![
            finding("Vincent Bolloré", 0),
            finding("Bolloré", 30),
            finding("Bollorés", 60),
        ];
        let groups = build_groups(&findings);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].finding_ids.len(), 3);
    }

    #[test]
    fn does_not_group_different_people_with_same_surname() {
        let findings = vec![finding("John Smith", 0), finding("Mary Smith", 20)];
        let groups = build_groups(&findings);
        assert_eq!(groups.len(), 2);
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
