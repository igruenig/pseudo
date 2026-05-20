use pseudo_core::{
    apply_replacements, build_groups, detect_deterministic, resolve_overlaps, AnalysisResult,
    Finding,
};
use sha2::{Digest, Sha256};

use crate::model_runtime::ModelRuntime;

pub async fn analyze(request_id: String, text: String, runtime: &ModelRuntime) -> AnalysisResult {
    let mut findings = detect_deterministic(&text);
    match runtime.detect(&text).await {
        Ok(mut llm_findings) => findings.append(&mut llm_findings),
        Err(error) => {
            let groups = build_groups(&findings);
            return AnalysisResult {
                request_id,
                source_text_hash: hash_text(&text),
                pseudonymized_text: apply_replacements(&text, &findings, &groups),
                findings,
                groups,
                warnings: vec![error.to_string()],
            };
        }
    }

    let findings = resolve_overlaps(findings);
    finish(request_id, text, findings, Vec::new())
}

pub fn recompute(
    request_id: String,
    text: String,
    findings: Vec<Finding>,
    warnings: Vec<String>,
) -> AnalysisResult {
    finish(request_id, text, resolve_overlaps(findings), warnings)
}

pub fn hash_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn finish(
    request_id: String,
    text: String,
    findings: Vec<Finding>,
    warnings: Vec<String>,
) -> AnalysisResult {
    let groups = build_groups(&findings);
    AnalysisResult {
        request_id,
        source_text_hash: hash_text(&text),
        pseudonymized_text: apply_replacements(&text, &findings, &groups),
        findings,
        groups,
        warnings,
    }
}
