use pseudo_core::{Finding, FindingSource, SensitiveType};
use serde::Deserialize;
use uuid::Uuid;

use pseudo_core::chunking::TextChunk;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmResponse {
    pub findings: Vec<LlmFinding>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmFinding {
    pub chunk_index: usize,
    pub text: String,
    pub r#type: SensitiveType,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub reason: Option<String>,
}

pub fn response_schema_json() -> &'static str {
    r#"{
      "type": "object",
      "properties": {
        "findings": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "chunkIndex": { "type": "integer" },
              "text": { "type": "string" },
              "type": {
                "type": "string",
                "enum": [
                  "PERSON_NAME",
                  "ORGANIZATION",
                  "ROLE_OR_POSITION",
                  "LOCATION",
                  "EMAIL",
                  "PHONE",
                  "DATE",
                  "ID_NUMBER",
                  "URL",
                  "OTHER_SENSITIVE"
                ]
              },
              "confidence": { "type": "number" },
              "reason": { "type": "string" }
            },
            "required": ["chunkIndex", "text", "type", "confidence"],
            "additionalProperties": false
          }
        },
        "warnings": {
          "type": "array",
          "items": { "type": "string" }
        }
      },
      "required": ["findings"],
      "additionalProperties": false
    }"#
}

pub fn response_grammar() -> Result<String, String> {
    llama_cpp_2::json_schema_to_grammar(response_schema_json()).map_err(|error| error.to_string())
}

pub fn parse_llm_response(json: &str, chunks: &[TextChunk]) -> Result<(Vec<Finding>, Vec<String>), String> {
    let json = extract_json_object(json).ok_or_else(|| "LLM did not return a JSON object".to_string())?;
    let response: LlmResponse = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let mut findings = Vec::new();
    let mut warnings = response.warnings;

    for item in response.findings {
        let Some(chunk) = chunks.iter().find(|chunk| chunk.chunk_index == item.chunk_index) else {
            warnings.push(format!("LLM returned unknown chunk index {}", item.chunk_index));
            continue;
        };
        if item.text.trim().is_empty() {
            warnings.push("LLM returned an empty finding".into());
            continue;
        }
        let mut matched = false;
        for (relative_start, _) in chunk.text.match_indices(&item.text) {
            matched = true;
            let confidence = item.confidence.unwrap_or(0.5).clamp(0.0, 1.0);
            findings.push(Finding {
                id: Uuid::new_v4().to_string(),
                r#type: item.r#type,
                start: chunk.start + relative_start,
                end: chunk.start + relative_start + item.text.len(),
                text: item.text.clone(),
                source: FindingSource::Llm,
                confidence: Some(confidence),
                needs_review: Some(confidence < 0.70 || item.r#type == SensitiveType::OtherSensitive),
            });
        }
        if !matched {
            let reason = item.reason.unwrap_or_else(|| "surface form not found in source chunk".into());
            warnings.push(format!("Discarded `{}`: {}", item.text, reason));
        }
    }

    Ok((findings, warnings))
}

fn extract_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let start = bytes.iter().position(|byte| *byte == b'{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, byte) in bytes[start..].iter().copied().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return text.get(start..start + offset + 1);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_builds_from_schema() {
        let grammar = response_grammar().expect("schema converts to grammar");
        assert!(grammar.contains("root"));
    }

    #[test]
    fn maps_surface_forms_to_source_offsets() {
        let source = "Jane called Jane Doe.";
        let chunks = vec![TextChunk {
            chunk_index: 0,
            start: 0,
            end: source.len(),
            text: source.into(),
        }];
        let json = r#"{"findings":[{"chunkIndex":0,"text":"Jane","type":"PERSON_NAME","confidence":0.82}]}"#;
        let (findings, warnings) = parse_llm_response(json, &chunks).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].start, 0);
        assert_eq!(findings[1].start, 12);
    }

    #[test]
    fn discards_surface_forms_not_in_chunk() {
        let chunks = vec![TextChunk {
            chunk_index: 0,
            start: 0,
            end: 5,
            text: "Hello".into(),
        }];
        let json = r#"{"findings":[{"chunkIndex":0,"text":"Jane","type":"PERSON_NAME","confidence":0.82}]}"#;
        let (findings, warnings) = parse_llm_response(json, &chunks).unwrap();
        assert!(findings.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn extracts_json_from_surrounding_text() {
        let source = "Jane called.";
        let chunks = vec![TextChunk {
            chunk_index: 0,
            start: 0,
            end: source.len(),
            text: source.into(),
        }];
        let json = r#"Here is JSON:
{"findings":[{"chunkIndex":0,"text":"Jane","type":"PERSON_NAME","confidence":0.82}]}
done"#;
        let (findings, warnings) = parse_llm_response(json, &chunks).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(findings.len(), 1);
    }
}
