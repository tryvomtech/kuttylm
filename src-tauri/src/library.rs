//! Downloaded models on disk.
//!
//! Each model has its own folder `models/<id>/` with:
//! - the GGUF files (`<name>.part` while downloading)
//! - `download.json` while the download is not finished
//! - `model.json` when all files are complete and checked

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::catalog::types::{is_safe_name, CatalogModel};
use crate::disk::{self, DiskSpace};
use crate::download::DownloadManager;
use crate::engine::Engine;
use crate::error::{AppError, AppResult};
use crate::paths;

/// Written when every file is complete and checked.
pub const READY_FILE: &str = "model.json";
/// Written when a download starts. Keeps the same files for Resume, even after a restart.
pub const PENDING_FILE: &str = "download.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(flatten)]
    pub model: CatalogModel,
    /// Unix seconds.
    pub saved_at: u64,
}

pub fn model_dir(id: &str) -> AppResult<PathBuf> {
    if !is_safe_name(id) {
        return Err(AppError::Internal(format!("bad model id {id:?}")));
    }
    Ok(paths::models_dir().join(id))
}

pub fn is_ready(dir: &Path) -> bool {
    dir.join(READY_FILE).is_file()
}

pub fn read_manifest(dir: &Path, file: &str) -> Option<Manifest> {
    let text = fs::read_to_string(dir.join(file)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Temp file + rename, so a crash never leaves half a file.
pub fn write_manifest(dir: &Path, file: &str, manifest: &Manifest) -> io::Result<()> {
    let path = dir.join(file);
    let tmp = dir.join(format!("{file}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(manifest)?)?;
    fs::rename(tmp, path)
}

// ---------- list ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalStatus {
    Ready,
    /// Paused, stopped by an error, or the app was closed during the download.
    Incomplete,
}

#[derive(Debug, Serialize)]
pub struct LocalModel {
    #[serde(flatten)]
    pub model: CatalogModel,
    pub status: LocalStatus,
    /// Everything in the model folder, including unfinished `.part` files.
    pub bytes_on_disk: u64,
    pub total_bytes: u64,
    pub saved_at: u64,
}

#[derive(Debug, Serialize)]
pub struct Library {
    pub models: Vec<LocalModel>,
    /// All models together.
    pub used_bytes: u64,
    /// Space of the disk that holds the models folder. `None` if unknown.
    pub disk: Option<DiskSpace>,
    pub models_dir: String,
}

pub fn scan() -> Library {
    let root = paths::models_dir();
    let mut models = Vec::new();

    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !dir.is_dir() || !is_safe_name(&name) {
                continue;
            }
            let (manifest, status) = match read_manifest(&dir, READY_FILE) {
                Some(m) => (m, LocalStatus::Ready),
                None => match read_manifest(&dir, PENDING_FILE) {
                    Some(m) => (m, LocalStatus::Incomplete),
                    None => continue, // Not a KuttyLM model folder.
                },
            };
            models.push(LocalModel {
                total_bytes: manifest.model.total_size_bytes(),
                bytes_on_disk: folder_size(&dir),
                model: manifest.model,
                status,
                saved_at: manifest.saved_at,
            });
        }
    }

    // Newest first.
    models.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
    Library {
        used_bytes: models.iter().map(|m| m.bytes_on_disk).sum(),
        models,
        disk: disk::space_for(&root),
        models_dir: root.to_string_lossy().into_owned(),
    }
}

fn folder_size(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.metadata().ok())
                .filter(|m| m.is_file())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

#[tauri::command]
pub async fn list_local_models() -> AppResult<Library> {
    tauri::async_runtime::spawn_blocking(scan)
        .await
        .map_err(|e| AppError::Internal(format!("could not read models folder: {e}")))
}

// ---------- delete ----------

#[tauri::command]
pub async fn delete_model(
    app: tauri::AppHandle,
    model_id: String,
    downloads: State<'_, DownloadManager>,
    engine: State<'_, Engine>,
) -> AppResult<()> {
    let dir = model_dir(&model_id)?;
    // A running model keeps its files open. Stop it first.
    engine.stop_if(&app, &model_id).await;
    // A running download removes its own folder when it stops.
    if downloads.cancel(&model_id) {
        return Ok(());
    }
    remove_model_dir(&dir)
}

pub fn remove_model_dir(dir: &Path) -> AppResult<()> {
    match fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Disk(e.to_string())),
    }
}
