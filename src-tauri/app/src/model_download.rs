use std::sync::Arc;

use futures_util::StreamExt;
use pseudo_core::{ModelDownloadState, ModelDownloadStatus};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::model_runtime::model_path;

const MODEL_NAME: &str = "Qwen3-1.7B Q4_K_M";
const MODEL_URL: &str = "https://huggingface.co/ggml-org/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q4_K_M.gguf";
const EXPECTED_BYTES: u64 = 1_282_439_264;

#[derive(Default)]
pub struct ModelDownloader {
    status: Arc<Mutex<Option<ModelDownloadStatus>>>,
}

impl ModelDownloader {
    pub async fn status(&self) -> ModelDownloadStatus {
        if let Some(status) = self.status.lock().await.clone() {
            return status;
        }
        let destination = model_path();
        let state = if destination.exists() {
            ModelDownloadState::Complete
        } else {
            ModelDownloadState::NotStarted
        };
        ModelDownloadStatus {
            state,
            model_name: MODEL_NAME.into(),
            destination_path: destination.display().to_string(),
            bytes_downloaded: 0,
            total_bytes: None,
            error: None,
        }
    }

    pub async fn start(&self) -> ModelDownloadStatus {
        let destination = model_path();
        if let Some(parent) = destination.parent() {
            if let Err(error) = tokio::fs::create_dir_all(parent).await {
                return self.error(error.to_string()).await;
            }
        }

        let mut status = ModelDownloadStatus {
            state: ModelDownloadState::Downloading,
            model_name: MODEL_NAME.into(),
            destination_path: destination.display().to_string(),
            bytes_downloaded: 0,
            total_bytes: Some(EXPECTED_BYTES),
            error: None,
        };
        *self.status.lock().await = Some(status.clone());

        let temp = destination.with_extension("download");
        let response = match reqwest::get(MODEL_URL).await {
            Ok(response) => response,
            Err(error) => return self.error(error.to_string()).await,
        };
        if !response.status().is_success() {
            return self
                .error(format!("model download failed with HTTP {}", response.status()))
                .await;
        }

        let total = response.content_length().or(Some(EXPECTED_BYTES));
        status.total_bytes = total;
        *self.status.lock().await = Some(status.clone());

        let mut file = match tokio::fs::File::create(&temp).await {
            Ok(file) => file,
            Err(error) => return self.error(error.to_string()).await,
        };
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => return self.error(error.to_string()).await,
            };
            if let Err(error) = file.write_all(&chunk).await {
                return self.error(error.to_string()).await;
            }
            status.bytes_downloaded += chunk.len() as u64;
            *self.status.lock().await = Some(status.clone());
        }
        if let Err(error) = file.flush().await {
            return self.error(error.to_string()).await;
        }

        if status.bytes_downloaded != EXPECTED_BYTES {
            let _ = tokio::fs::remove_file(&temp).await;
            return self
                .error(format!(
                    "downloaded file size was {}, expected {}",
                    status.bytes_downloaded, EXPECTED_BYTES
                ))
                .await;
        }
        if let Err(error) = tokio::fs::rename(&temp, &destination).await {
            return self.error(error.to_string()).await;
        }

        status.state = ModelDownloadState::Complete;
        *self.status.lock().await = Some(status.clone());
        status
    }

    pub async fn cancel(&self) -> ModelDownloadStatus {
        let mut status = self.status().await;
        status.state = ModelDownloadState::Cancelled;
        *self.status.lock().await = Some(status.clone());
        status
    }

    async fn error(&self, message: String) -> ModelDownloadStatus {
        let mut status = self.status().await;
        status.state = ModelDownloadState::Error;
        status.error = Some(message);
        *self.status.lock().await = Some(status.clone());
        status
    }
}
