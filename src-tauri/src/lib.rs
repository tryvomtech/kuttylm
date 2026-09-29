mod catalog;
mod config;
mod device;
mod disk;
mod download;
mod engine;
mod error;
mod fit;
mod format;
mod hf;
mod library;
mod net;
mod paths;
mod platform;
mod settings;
mod util;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(catalog::CatalogState::default())
        .manage(download::DownloadManager::default())
        .manage(engine::Engine::default())
        .manage(hf::HfState::default())
        .invoke_handler(tauri::generate_handler![
            device::get_device_info,
            catalog::get_models,
            download::start_download,
            download::pause_download,
            download::cancel_download,
            library::list_local_models,
            library::delete_model,
            engine::get_engine_status,
            engine::start_model,
            engine::stop_model,
            settings::get_settings,
            settings::save_settings,
            settings::get_network_info,
            settings::get_app_paths,
            hf::hf_search,
            hf::hf_repo_files,
        ])
        .setup(|_app| {
            // Debug builds only: `KUTTYLM_DEV_AUTORUN=<model-id>` starts a model at launch,
            // so the start/stop/exit path can be tested without clicking.
            #[cfg(debug_assertions)]
            if let Ok(model_id) = std::env::var("KUTTYLM_DEV_AUTORUN") {
                let handle = _app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let engine = handle.state::<engine::Engine>();
                    if let Err(e) = engine::start_model(handle.clone(), model_id, engine).await {
                        eprintln!("[dev] autorun failed: {e}");
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building KuttyLM");

    app.run(|app, event| {
        // Closing the window ends the app. Never leave llama-server running.
        if let RunEvent::Exit = event {
            app.state::<engine::Engine>().shutdown_blocking();
        }
    });
}
