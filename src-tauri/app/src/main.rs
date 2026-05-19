mod analysis;
mod commands;
mod model_download;
mod model_runtime;

fn main() {
    tauri::Builder::default()
        .manage(model_runtime::ModelRuntime::default())
        .manage(model_download::ModelDownloader::default())
        .manage(commands::AnalysisCache::default())
        .invoke_handler(tauri::generate_handler![
            commands::analyze_text,
            commands::apply_replacements,
            commands::create_manual_finding,
            commands::recompute_analysis,
            commands::load_model,
            commands::unload_model,
            commands::get_model_status,
            commands::get_model_download_status,
            commands::start_model_download,
            commands::cancel_model_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
