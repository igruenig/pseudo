use std::{num::NonZeroU32, path::PathBuf, sync::Arc, time::Instant};

use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::AddBos,
    model::{params::LlamaModelParams, LlamaModel},
    sampling::LlamaSampler,
};
use pseudo_core::chunking::{chunk_text_for_analysis, TextChunk};
use pseudo_core::{Finding, ModelBackend, ModelStatus};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::llm_output::parse_llm_response;

const CONTEXT_TOKENS: u32 = 8_192;
const MAX_NEW_TOKENS: i32 = 1_024;
const PROMPT_TOKEN_MARGIN: usize = 64;
const CHUNK_FALLBACK_CHARS: usize = 6_000;

#[derive(Default)]
pub struct ModelRuntime {
    state: Arc<Mutex<Option<LoadedModel>>>,
}

struct LoadedModel {
    backend: LlamaBackend,
    model: LlamaModel,
    path: PathBuf,
    load_ms: u64,
}

#[derive(Debug, Error)]
pub enum ModelRuntimeError {
    #[error("Local model unavailable. Download the model before LLM-assisted analysis.")]
    MissingModel,
    #[error("Failed to initialize llama.cpp backend: {0}")]
    Backend(String),
    #[error("Failed to load local model: {0}")]
    Load(String),
    #[error("Failed to create llama.cpp context: {0}")]
    Context(String),
    #[error("Failed to tokenize prompt: {0}")]
    Tokenize(String),
    #[error("Failed to decode model tokens: {0}")]
    Decode(String),
    #[error("Failed to parse model output: {0}")]
    Parse(String),
}

impl ModelRuntime {
    pub async fn load(&self) -> Result<ModelStatus, ModelRuntimeError> {
        if let Some(loaded) = self.state.lock().await.as_ref() {
            return Ok(loaded.status());
        }

        let path = model_path();
        if !path.exists() {
            return Err(ModelRuntimeError::MissingModel);
        }

        let loaded = {
            let started = Instant::now();
            let mut backend = LlamaBackend::init()
                .map_err(|error| ModelRuntimeError::Backend(error.to_string()))?;
            backend.void_logs();
            let model = {
                let params = model_params();
                LlamaModel::load_from_file(&backend, &path, &params)
                    .map_err(|error| ModelRuntimeError::Load(error.to_string()))?
            };
            let load_ms = started.elapsed().as_millis() as u64;

            LoadedModel {
                backend,
                model,
                path,
                load_ms,
            }
        };
        let status = loaded.status();
        *self.state.lock().await = Some(loaded);
        Ok(status)
    }

    pub async fn unload(&self) {
        *self.state.lock().await = None;
    }

    pub async fn status(&self) -> ModelStatus {
        self.state
            .lock()
            .await
            .as_ref()
            .map(LoadedModel::status)
            .unwrap_or_else(|| status_for(None, None))
    }

    pub async fn detect(&self, text: &str) -> Result<Vec<Finding>, ModelRuntimeError> {
        if self.state.lock().await.is_none() {
            self.load().await?;
        }
        let guard = self.state.lock().await;
        let loaded = guard.as_ref().ok_or(ModelRuntimeError::MissingModel)?;
        loaded.detect(text)
    }
}

pub fn model_path() -> PathBuf {
    if let Ok(path) = std::env::var("PSEUDO_MODEL_PATH") {
        return PathBuf::from(path);
    }
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("pseudo")
        .join("models")
        .join("qwen3-1.7b-q4_k_m.gguf")
}

impl LoadedModel {
    fn status(&self) -> ModelStatus {
        let _model_size_bytes = self.model.size();
        status_for(Some(self.path.clone()), Some(self.load_ms))
    }

    fn detect(&self, text: &str) -> Result<Vec<Finding>, ModelRuntimeError> {
        let chunks = self.chunks_for_prompt(text)?;
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let prompt = build_prompt(&chunks);
        let output = self.generate_json(&prompt, MAX_NEW_TOKENS)?;
        let (findings, _warnings) =
            parse_llm_response(&output, &chunks).map_err(ModelRuntimeError::Parse)?;
        Ok(findings)
    }

    fn chunks_for_prompt(&self, text: &str) -> Result<Vec<TextChunk>, ModelRuntimeError> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let whole_document = vec![TextChunk {
            chunk_index: 0,
            start: 0,
            end: text.len(),
            text: text.to_string(),
        }];
        if self.prompt_fits_context(&build_prompt(&whole_document))? {
            return Ok(whole_document);
        }

        Ok(chunk_text_for_analysis(text, CHUNK_FALLBACK_CHARS))
    }

    fn prompt_fits_context(&self, prompt: &str) -> Result<bool, ModelRuntimeError> {
        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|error| ModelRuntimeError::Tokenize(error.to_string()))?;
        Ok(tokens.len() + MAX_NEW_TOKENS as usize + PROMPT_TOKEN_MARGIN <= CONTEXT_TOKENS as usize)
    }

    fn generate_json(
        &self,
        prompt: &str,
        max_new_tokens: i32,
    ) -> Result<String, ModelRuntimeError> {
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(CONTEXT_TOKENS).expect("nonzero")));
        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|error| ModelRuntimeError::Context(error.to_string()))?;
        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|error| ModelRuntimeError::Tokenize(error.to_string()))?;
        if tokens.is_empty() {
            return Ok(String::new());
        }

        let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
        let last_index = tokens.len() - 1;
        for (index, token) in tokens.iter().copied().enumerate() {
            batch
                .add(token, index as i32, &[0], index == last_index)
                .map_err(|error| ModelRuntimeError::Decode(error.to_string()))?;
        }
        ctx.decode(&mut batch)
            .map_err(|error| ModelRuntimeError::Decode(error.to_string()))?;

        let mut sampler = LlamaSampler::greedy();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        let mut position = batch.n_tokens();

        for _ in 0..max_new_tokens {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);
            if self.model.is_eog_token(token) {
                break;
            }
            let piece = self
                .model
                .token_to_piece(token, &mut decoder, true, None)
                .map_err(|error| ModelRuntimeError::Decode(error.to_string()))?;
            output.push_str(&piece);
            batch.clear();
            batch
                .add(token, position, &[0], true)
                .map_err(|error| ModelRuntimeError::Decode(error.to_string()))?;
            position += 1;
            ctx.decode(&mut batch)
                .map_err(|error| ModelRuntimeError::Decode(error.to_string()))?;
        }

        Ok(output)
    }
}

fn build_prompt(chunks: &[TextChunk]) -> String {
    let chunk_json = chunks
        .iter()
        .map(|chunk| {
            serde_json::json!({
                "chunkIndex": chunk.chunk_index,
                "text": chunk.text,
            })
        })
        .collect::<Vec<_>>();

    format!(
        "<|im_start|>system\n\
You extract sensitive spans for local pseudonymization from English, German, French, and mixed-language text. Return JSON only. Do not explain.\n\
Read the full input before deciding. Use global document context to catch repeated, shortened, or inflected mentions of the same person or organization.\n\
Every finding text must be an exact substring copied from the input chunk.\n\
Do not infer hidden values. Do not extract parts inside an email if the full email is already extracted.\n\
Always extract visible personal names, including names with accents or non-English characters. If a person is first named fully and later mentioned by surname or a possessive/inflected surname, extract those later mentions too.\n\
Always extract cities, countries, venues, and named events when they identify context, including city names at the beginning of a document.\n\
Detect organizations, including company names with legal suffixes such as AG, GmbH, Ltd, LLC, Inc, SA, or BV.\n\
Do not extract standalone ordinal/cardinal numbers, generic nouns, broad topic words, or descriptive nouns such as Milliardär unless they are part of a longer identifying title/name span.\n\
Valid types: PERSON_NAME, ORGANIZATION, ROLE_OR_POSITION, LOCATION, EMAIL, PHONE, DATE, ID_NUMBER, URL, OTHER_SENSITIVE.\n\
Required output shape: {{\"findings\":[{{\"chunkIndex\":0,\"text\":\"exact substring from input\",\"type\":\"PERSON_NAME\",\"confidence\":0.90}}]}}\n\
If there are no findings, return {{\"findings\":[]}}.\n\
<|im_end|>\n\
<|im_start|>user\n\
/no_think\n\
Input chunks. Normal-length documents are sent as one chunk; multiple chunks mean the document was too long for one prompt:\n{}\n\
Return JSON now.\n\
<|im_end|>\n\
<|im_start|>assistant\n",
        serde_json::to_string(&chunk_json).expect("chunk json serializes")
    )
}

fn status_for(path: Option<PathBuf>, load_ms: Option<u64>) -> ModelStatus {
    ModelStatus {
        loaded: path.is_some(),
        model_path: path.map(|path| path.display().to_string()),
        quantization: Some("Q4_K_M".into()),
        backend: runtime_backend(),
        load_ms,
        resident_memory_mb: None,
    }
}

#[cfg(target_os = "macos")]
fn model_params() -> LlamaModelParams {
    LlamaModelParams::default().with_n_gpu_layers(u32::MAX)
}

#[cfg(not(target_os = "macos"))]
fn model_params() -> LlamaModelParams {
    LlamaModelParams::default()
}

#[cfg(target_os = "macos")]
fn runtime_backend() -> ModelBackend {
    ModelBackend::Metal
}

#[cfg(not(target_os = "macos"))]
fn runtime_backend() -> ModelBackend {
    ModelBackend::Cpu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "loads the local Qwen GGUF model"]
    fn detects_with_local_model() {
        tauri::async_runtime::block_on(async {
            let runtime = ModelRuntime::default();
            let text = "In Cannes laufen die 79. Internationalen Filmfestspiele. Neben dem üblichen Glamour und den vielen Talenten sorgen dieses Jahr auch polarisierende Themen für Schlagzeilen. Die Debatte um den ultrakonservativen Milliardär Vincent Bolloré und dessen Einfluss auf die Kulturszene geht in eine neue Runde.";
            let result = crate::analysis::analyze("smoke".into(), text.into(), &runtime).await;
            assert_eq!(result.request_id, "smoke");
            assert!(result
                .findings
                .iter()
                .any(|finding| finding.text == "Vincent Bolloré"));
            assert!(!result.findings.iter().any(|finding| finding.text == "79."));
            assert!(!result
                .findings
                .iter()
                .any(|finding| finding.text == "Filmfestspiele"));
            assert!(!result
                .findings
                .iter()
                .any(|finding| finding.text == "Milliardär"));
            assert!(!result
                .findings
                .iter()
                .any(|finding| finding.text == "Kulturszene"));
        });
    }
}
