use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensitiveType {
    PersonName,
    Organization,
    RoleOrPosition,
    Location,
    Email,
    Phone,
    Date,
    IdNumber,
    Url,
    OtherSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingSource {
    Deterministic,
    Llm,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub r#type: SensitiveType,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub source: FindingSource,
    pub confidence: Option<f32>,
    pub needs_review: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementGroup {
    pub id: String,
    pub r#type: SensitiveType,
    pub original: String,
    pub normalized_original: String,
    pub replacement: String,
    pub finding_ids: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub request_id: String,
    pub source_text_hash: String,
    pub findings: Vec<Finding>,
    pub groups: Vec<ReplacementGroup>,
    pub pseudonymized_text: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub loaded: bool,
    pub model_path: Option<String>,
    pub quantization: Option<String>,
    pub backend: ModelBackend,
    pub load_ms: Option<u64>,
    pub resident_memory_mb: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelBackend {
    Metal,
    Cpu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadStatus {
    pub state: ModelDownloadState,
    pub model_name: String,
    pub destination_path: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDownloadState {
    NotStarted,
    Downloading,
    Complete,
    Error,
    Cancelled,
}
