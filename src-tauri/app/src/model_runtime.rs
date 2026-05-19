use std::{path::PathBuf, sync::Arc};

use pseudo_core::{Finding, ModelBackend, ModelStatus};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct ModelRuntime {
    state: Arc<Mutex<Option<PathBuf>>>,
}

#[derive(Debug, Error)]
pub enum ModelRuntimeError {
    #[error("Local model unavailable. Download the model before LLM-assisted analysis.")]
    MissingModel,
}

impl ModelRuntime {
    pub async fn load(&self) -> Result<ModelStatus, ModelRuntimeError> {
        let path = model_path();
        if !path.exists() {
            return Err(ModelRuntimeError::MissingModel);
        }
        *self.state.lock().await = Some(path.clone());
        Ok(status_for(Some(path)))
    }

    pub async fn unload(&self) {
        *self.state.lock().await = None;
    }

    pub async fn status(&self) -> ModelStatus {
        let loaded = self.state.lock().await.clone();
        status_for(loaded)
    }

    pub async fn detect(&self, _text: &str) -> Result<Vec<Finding>, ModelRuntimeError> {
        if self.state.lock().await.is_none() {
            self.load().await?;
        }
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

fn status_for(path: Option<PathBuf>) -> ModelStatus {
    ModelStatus {
        loaded: path.is_some(),
        model_path: path.map(|path| path.display().to_string()),
        quantization: Some("Q4_K_M".into()),
        backend: if cfg!(target_os = "macos") {
            ModelBackend::Metal
        } else {
            ModelBackend::Cpu
        },
        load_ms: None,
        resident_memory_mb: None,
    }
}
