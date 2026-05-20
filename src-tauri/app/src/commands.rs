use pseudo_core::{
    apply_replacements as apply_core_replacements,
    create_manual_finding as create_core_manual_finding, AnalysisResult, Finding,
    ModelDownloadStatus, ModelStatus, ReplacementGroup, SensitiveType,
};
use tauri::State;
use tokio::sync::Mutex;

use crate::{
    analysis,
    model_download::ModelDownloader,
    model_runtime::{ModelRuntime, ModelRuntimeError},
};

#[derive(Default)]
pub struct AnalysisCache {
    last: Mutex<Option<AnalysisResult>>,
}

#[tauri::command]
pub async fn analyze_text(
    request_id: String,
    text: String,
    runtime: State<'_, ModelRuntime>,
    cache: State<'_, AnalysisCache>,
) -> Result<AnalysisResult, String> {
    let result = analysis::analyze(request_id, text, &runtime).await;
    *cache.last.lock().await = Some(result.clone());
    Ok(result)
}

#[tauri::command]
pub async fn apply_replacements(
    text: String,
    expected_text_hash: String,
    groups: Vec<ReplacementGroup>,
    cache: State<'_, AnalysisCache>,
) -> Result<String, String> {
    if analysis::hash_text(&text) != expected_text_hash {
        return Err("Current text no longer matches the analyzed text.".into());
    }
    let guard = cache.last.lock().await;
    let Some(result) = guard.as_ref() else {
        return Err("No cached analysis is available.".into());
    };
    if result.source_text_hash != expected_text_hash {
        return Err("Cached analysis no longer matches the requested text.".into());
    }
    Ok(apply_core_replacements(&text, &result.findings, &groups))
}

#[tauri::command]
pub async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_manual_finding(
    _request_id: String,
    text: String,
    start: usize,
    end: usize,
    r#type: SensitiveType,
) -> Result<Finding, String> {
    create_core_manual_finding(&text, start, end, r#type).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn recompute_analysis(
    request_id: String,
    text: String,
    expected_text_hash: String,
    findings: Vec<Finding>,
    _groups: Vec<ReplacementGroup>,
    cache: State<'_, AnalysisCache>,
) -> Result<AnalysisResult, String> {
    if analysis::hash_text(&text) != expected_text_hash {
        return Err("Current text no longer matches the analyzed text.".into());
    }
    let result = analysis::recompute(request_id, text, findings, Vec::new());
    *cache.last.lock().await = Some(result.clone());
    Ok(result)
}

#[tauri::command]
pub async fn load_model(runtime: State<'_, ModelRuntime>) -> Result<ModelStatus, String> {
    runtime.load().await.map_err(runtime_error)
}

#[tauri::command]
pub async fn unload_model(runtime: State<'_, ModelRuntime>) -> Result<(), String> {
    runtime.unload().await;
    Ok(())
}

#[tauri::command]
pub async fn get_model_status(runtime: State<'_, ModelRuntime>) -> Result<ModelStatus, String> {
    Ok(runtime.status().await)
}

#[tauri::command]
pub async fn get_model_download_status(
    downloader: State<'_, ModelDownloader>,
) -> Result<ModelDownloadStatus, String> {
    Ok(downloader.status().await)
}

#[tauri::command]
pub async fn start_model_download(
    downloader: State<'_, ModelDownloader>,
) -> Result<ModelDownloadStatus, String> {
    Ok(downloader.start().await)
}

#[tauri::command]
pub async fn cancel_model_download(
    downloader: State<'_, ModelDownloader>,
) -> Result<ModelDownloadStatus, String> {
    Ok(downloader.cancel().await)
}

fn runtime_error(error: ModelRuntimeError) -> String {
    error.to_string()
}
