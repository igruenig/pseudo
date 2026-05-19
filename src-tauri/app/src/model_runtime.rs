use std::{
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use llama_cpp_2::{
    llama_backend::LlamaBackend,
    model::{params::LlamaModelParams, LlamaModel},
};
use pseudo_core::chunking::chunk_text_for_analysis;
use pseudo_core::{Finding, ModelBackend, ModelStatus};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::llm_output::response_grammar;

#[derive(Default)]
pub struct ModelRuntime {
    state: Arc<Mutex<Option<LoadedModel>>>,
}

struct LoadedModel {
    _backend: LlamaBackend,
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
    #[error("Failed to build model output grammar: {0}")]
    Grammar(String),
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
            let mut backend =
                LlamaBackend::init().map_err(|error| ModelRuntimeError::Backend(error.to_string()))?;
            backend.void_logs();
            let model = {
                let params = LlamaModelParams::default();
                LlamaModel::load_from_file(&backend, &path, &params)
                    .map_err(|error| ModelRuntimeError::Load(error.to_string()))?
            };
            let load_ms = started.elapsed().as_millis() as u64;

            LoadedModel {
                _backend: backend,
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

    pub async fn detect(&self, _text: &str) -> Result<Vec<Finding>, ModelRuntimeError> {
        if self.state.lock().await.is_none() {
            self.load().await?;
        }
        let _grammar = response_grammar().map_err(ModelRuntimeError::Grammar)?;
        let _chunks = chunk_text_for_analysis(_text, 2_000);
        // The next implementation slice creates a context, applies the JSON grammar,
        // and converts model-returned surface forms into source ranges.
        Ok(Vec::new())
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
}

fn status_for(path: Option<PathBuf>, load_ms: Option<u64>) -> ModelStatus {
    ModelStatus {
        loaded: path.is_some(),
        model_path: path.map(|path| path.display().to_string()),
        quantization: Some("Q4_K_M".into()),
        backend: ModelBackend::Cpu,
        load_ms,
        resident_memory_mb: None,
    }
}
