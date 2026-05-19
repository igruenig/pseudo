use crate::types::{Finding, FindingSource, SensitiveType};

pub fn resolve_overlaps(mut findings: Vec<Finding>) -> Vec<Finding> {
    findings.sort_by(compare_priority);
    let mut accepted: Vec<Finding> = Vec::new();

    'candidate: for candidate in findings {
        if accepted.iter().any(|existing| overlaps(existing, &candidate)) {
            continue 'candidate;
        }
        accepted.push(candidate);
    }

    accepted.sort_by_key(|finding| (finding.start, finding.end));
    accepted
}

fn compare_priority(a: &Finding, b: &Finding) -> std::cmp::Ordering {
    source_rank(b.source)
        .cmp(&source_rank(a.source))
        .then_with(|| confidence(b).total_cmp(&confidence(a)))
        .then_with(|| (b.end - b.start).cmp(&(a.end - a.start)))
        .then_with(|| a.start.cmp(&b.start))
        .then_with(|| type_rank(a.r#type).cmp(&type_rank(b.r#type)))
}

fn overlaps(a: &Finding, b: &Finding) -> bool {
    a.start < b.end && b.start < a.end
}

fn confidence(finding: &Finding) -> f32 {
    finding.confidence.unwrap_or(0.5).clamp(0.0, 1.0)
}

fn source_rank(source: FindingSource) -> u8 {
    match source {
        FindingSource::Manual => 3,
        FindingSource::Deterministic => 2,
        FindingSource::Llm => 1,
    }
}

fn type_rank(r#type: SensitiveType) -> u8 {
    match r#type {
        SensitiveType::Email => 0,
        SensitiveType::Url => 1,
        SensitiveType::IdNumber => 2,
        SensitiveType::Phone => 3,
        SensitiveType::Date => 4,
        SensitiveType::PersonName => 5,
        SensitiveType::Organization => 6,
        SensitiveType::RoleOrPosition => 7,
        SensitiveType::Location => 8,
        SensitiveType::OtherSensitive => 9,
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{FindingSource, SensitiveType};

    use super::*;

    #[test]
    fn manual_beats_llm_overlap() {
        let findings = vec![
            finding("llm", 0, 10, FindingSource::Llm),
            finding("manual", 2, 6, FindingSource::Manual),
        ];
        let resolved = resolve_overlaps(findings);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].text, "manual");
    }

    fn finding(text: &str, start: usize, end: usize, source: FindingSource) -> Finding {
        Finding {
            id: text.into(),
            r#type: SensitiveType::PersonName,
            start,
            end,
            text: text.into(),
            source,
            confidence: Some(0.8),
            needs_review: Some(false),
        }
    }
}
