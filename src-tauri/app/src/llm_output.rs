use std::collections::BTreeMap;

use pseudo_core::{Finding, FindingSource, SensitiveType};
use serde::{de, Deserialize, Deserializer};
use serde_json::Value;
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LlmEnvelope {
    Schema(LlmResponse),
    NumberedMap(BTreeMap<String, LegacyFinding>),
}

#[derive(Debug)]
struct LegacyFinding {
    text: String,
    r#type: SensitiveType,
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
    let envelope: LlmEnvelope = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let response = match envelope {
        LlmEnvelope::Schema(response) => response,
        LlmEnvelope::NumberedMap(items) => LlmResponse {
            findings: items
                .into_values()
                .map(|item| LlmFinding {
                    chunk_index: 0,
                    text: item.text,
                    r#type: item.r#type,
                    confidence: Some(0.6),
                    reason: None,
                })
                .collect(),
            warnings: vec!["Adapted non-schema LLM output".into()],
        },
    };
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
        for relative_start in surface_matches(&chunk.text, &item.text) {
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

impl<'de> Deserialize<'de> for LegacyFinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Some(items) = value.as_array() else {
            return Err(de::Error::custom("legacy finding must be an array"));
        };
        let text = items
            .first()
            .and_then(Value::as_str)
            .ok_or_else(|| de::Error::custom("legacy finding missing text"))?
            .to_string();
        let type_name = items
            .get(1)
            .and_then(Value::as_str)
            .ok_or_else(|| de::Error::custom("legacy finding missing type"))?;
        let r#type = parse_sensitive_type(type_name).map_err(de::Error::custom)?;
        Ok(Self { text, r#type })
    }
}

fn parse_sensitive_type(value: &str) -> Result<SensitiveType, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "PERSON" | "PERSON_NAME" | "NAME" => Ok(SensitiveType::PersonName),
        "COMPANY" | "ORG" | "ORGANIZATION" | "ORGANISATION" => Ok(SensitiveType::Organization),
        "ROLE" | "ROLE_OR_POSITION" | "POSITION" | "TITLE" => Ok(SensitiveType::RoleOrPosition),
        "LOCATION" | "ADDRESS" | "PLACE" => Ok(SensitiveType::Location),
        "EMAIL" | "E-MAIL" => Ok(SensitiveType::Email),
        "PHONE" | "TELEPHONE" | "TEL" => Ok(SensitiveType::Phone),
        "DATE" => Ok(SensitiveType::Date),
        "ID" | "ID_NUMBER" | "IDENTIFIER" => Ok(SensitiveType::IdNumber),
        "URL" | "LINK" | "WEBSITE" => Ok(SensitiveType::Url),
        "OTHER" | "OTHER_SENSITIVE" | "SENSITIVE" => Ok(SensitiveType::OtherSensitive),
        other => Err(format!("unknown sensitive type `{other}`")),
    }
}

fn surface_matches(haystack: &str, needle: &str) -> Vec<usize> {
    let exact = haystack.match_indices(needle).map(|(start, _)| start).collect::<Vec<_>>();
    if !exact.is_empty() {
        return exact;
    }

    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    haystack_lower
        .match_indices(&needle_lower)
        .filter_map(|(start, _)| haystack.is_char_boundary(start).then_some(start))
        .collect()
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

    #[test]
    fn adapts_numbered_map_output() {
        let source = "Jane Doe emailed jane.doe@example.com about ACME AG.";
        let chunks = vec![TextChunk {
            chunk_index: 0,
            start: 0,
            end: source.len(),
            text: source.into(),
        }];
        let json = r#"{"0":["jane.doe@example.com","email"],"1":["acme ag","company"]}"#;
        let (findings, warnings) = parse_llm_response(json, &chunks).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].r#type, SensitiveType::Email);
        assert_eq!(findings[1].text, "acme ag");
        assert_eq!(findings[1].start, 44);
        assert_eq!(findings[1].end, 51);
        assert_eq!(warnings, vec!["Adapted non-schema LLM output"]);
    }
}
