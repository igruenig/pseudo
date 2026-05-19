use std::io::Read;

use anyhow::Context;
use pseudo_core::{apply_replacements, build_groups, detect_deterministic, AnalysisResult};
use sha2::{Digest, Sha256};

fn main() -> anyhow::Result<()> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("failed to read stdin")?;

    let findings = detect_deterministic(&text);
    let groups = build_groups(&findings);
    let result = AnalysisResult {
        request_id: "cli".into(),
        source_text_hash: hash_text(&text),
        pseudonymized_text: apply_replacements(&text, &findings, &groups),
        findings,
        groups,
        warnings: Vec::new(),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn hash_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
