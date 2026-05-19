use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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
    cancel_requested: Arc<AtomicBool>,
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
        if destination.exists() {
            let status = complete_status(destination);
            *self.status.lock().await = Some(status.clone());
            return status;
        }

        if matches!(
            self.status.lock().await.as_ref().map(|status| status.state),
            Some(ModelDownloadState::Downloading)
        ) {
            return self.status().await;
        }

        if let Some(parent) = destination.parent() {
            if let Err(error) = tokio::fs::create_dir_all(parent).await {
                return self.error(error.to_string()).await;
            }
        }

        let status = ModelDownloadStatus {
            state: ModelDownloadState::Downloading,
            model_name: MODEL_NAME.into(),
            destination_path: destination.display().to_string(),
            bytes_downloaded: 0,
            total_bytes: Some(EXPECTED_BYTES),
            error: None,
        };
        self.cancel_requested.store(false, Ordering::Relaxed);
        *self.status.lock().await = Some(status.clone());

        let status_handle = Arc::clone(&self.status);
        let cancel_handle = Arc::clone(&self.cancel_requested);
        tokio::spawn(async move {
            download_model(destination, status_handle, cancel_handle).await;
        });

        status
    }

    pub async fn cancel(&self) -> ModelDownloadStatus {
        self.cancel_requested.store(true, Ordering::Relaxed);
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

async fn download_model(
    destination: std::path::PathBuf,
    status_handle: Arc<Mutex<Option<ModelDownloadStatus>>>,
    cancel_requested: Arc<AtomicBool>,
) {
    let mut status = ModelDownloadStatus {
        state: ModelDownloadState::Downloading,
        model_name: MODEL_NAME.into(),
        destination_path: destination.display().to_string(),
        bytes_downloaded: 0,
        total_bytes: Some(EXPECTED_BYTES),
        error: None,
    };

    let result = async {
        let temp = destination.with_extension("download");
        let response = reqwest::get(MODEL_URL).await.map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("model download failed with HTTP {}", response.status()));
        }

        let total = response.content_length().or(Some(EXPECTED_BYTES));
        status.total_bytes = total;
        *status_handle.lock().await = Some(status.clone());

        let mut file = tokio::fs::File::create(&temp)
            .await
            .map_err(|error| error.to_string())?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if cancel_requested.load(Ordering::Relaxed) {
                let _ = tokio::fs::remove_file(&temp).await;
                status.state = ModelDownloadState::Cancelled;
                *status_handle.lock().await = Some(status.clone());
                return Ok(());
            }
            let chunk = chunk.map_err(|error| error.to_string())?;
            file.write_all(&chunk)
                .await
                .map_err(|error| error.to_string())?;
            status.bytes_downloaded += chunk.len() as u64;
            *status_handle.lock().await = Some(status.clone());
        }
        file.flush().await.map_err(|error| error.to_string())?;

        if status.bytes_downloaded != EXPECTED_BYTES {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(format!(
                "downloaded file size was {}, expected {}",
                status.bytes_downloaded, EXPECTED_BYTES
            ));
        }
        tokio::fs::rename(&temp, &destination)
            .await
            .map_err(|error| error.to_string())?;

        status.state = ModelDownloadState::Complete;
        *status_handle.lock().await = Some(status.clone());
        Ok(())
    }
    .await;

    if let Err(error) = result {
        status.state = ModelDownloadState::Error;
        status.error = Some(error);
        *status_handle.lock().await = Some(status);
    }
}

fn complete_status(destination: std::path::PathBuf) -> ModelDownloadStatus {
    ModelDownloadStatus {
        state: ModelDownloadState::Complete,
        model_name: MODEL_NAME.into(),
        destination_path: destination.display().to_string(),
        bytes_downloaded: EXPECTED_BYTES,
        total_bytes: Some(EXPECTED_BYTES),
        error: None,
    }
}
