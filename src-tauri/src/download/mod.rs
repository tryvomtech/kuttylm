//! Model downloads. One download at a time.
//!
//! - Pause keeps the `.part` files. Resume (= start again) continues them.
//! - Cancel removes the model folder.
//! - Progress goes to the UI as the `download-progress` event.

mod progress;
mod task;
mod verify;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::watch;

use crate::catalog::types::CatalogModel;
use crate::catalog::CatalogState;
use crate::error::{AppError, AppResult};
use crate::hf::HfState;
use crate::library::{self, Manifest, PENDING_FILE, READY_FILE};
use crate::util::now_secs;
use crate::{config, disk};
pub use progress::{DownloadEvent, DownloadStatus, Progress};
use task::Outcome;

pub const EVENT: &str = "download-progress";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Run,
    Pause,
    Cancel,
}

struct Active {
    model_id: String,
    control: watch::Sender<Control>,
}

#[derive(Default)]
pub struct DownloadManager {
    active: Mutex<Option<Active>>,
}

impl DownloadManager {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Active>> {
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Sends a signal to the running download of `model_id`. `false` if it is not running.
    fn signal(&self, model_id: &str, control: Control) -> bool {
        match &*self.lock() {
            Some(a) if a.model_id == model_id => {
                a.control.send_replace(control);
                true
            }
            _ => false,
        }
    }

    /// `true` while a download is running.
    pub fn is_busy(&self) -> bool {
        self.lock().is_some()
    }

    /// Cancels a running download. The task removes the folder when it stops.
    pub fn cancel(&self, model_id: &str) -> bool {
        self.signal(model_id, Control::Cancel)
    }

    fn release(&self, model_id: &str) {
        let mut active = self.lock();
        if active.as_ref().is_some_and(|a| a.model_id == model_id) {
            *active = None;
        }
    }
}

fn emit(app: &AppHandle, event: DownloadEvent) {
    let _ = app.emit(EVENT, event);
}

fn simple_event(model: &CatalogModel, status: DownloadStatus, downloaded: u64, error: Option<String>) -> DownloadEvent {
    DownloadEvent {
        model_id: model.id.clone(),
        status,
        downloaded_bytes: downloaded,
        total_bytes: model.total_size_bytes(),
        speed_bps: 0,
        eta_secs: None,
        error,
    }
}

/// Starts a new download or resumes a paused one.
#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    model_id: String,
    catalog: State<'_, CatalogState>,
    hf: State<'_, HfState>,
    manager: State<'_, DownloadManager>,
) -> AppResult<()> {
    let dir = library::model_dir(&model_id)?;
    if library::is_ready(&dir) {
        return Ok(());
    }
    // Resume uses the saved file list, so a changed online list cannot mix files.
    let model = match library::read_manifest(&dir, PENDING_FILE) {
        Some(saved) => saved.model,
        None => catalog
            .find(&model_id)
            .or_else(|| hf.find(&model_id))
            .or_else(|| (model_id == crate::docs::embed::EMBED_MODEL_ID).then(crate::docs::embed::embed_model))
            .ok_or(AppError::UnknownModel)?,
    };

    let control = {
        let mut active = manager.lock();
        match &*active {
            Some(a) if a.model_id == model_id => return Ok(()),
            Some(_) => return Err(AppError::OtherDownloadRunning),
            None => {}
        }
        let (tx, rx) = watch::channel(Control::Run);
        *active = Some(Active { model_id: model_id.clone(), control: tx });
        rx
    };

    if let Err(e) = prepare(&model, &dir) {
        manager.release(&model_id);
        return Err(e);
    }

    tauri::async_runtime::spawn(async move {
        let progress_app = app.clone();
        let id = model.id.clone();
        let base_url = config::hf_base_url();
        let result = task::run(&model, &dir, &base_url, control, |p: Progress| {
            emit(
                &progress_app,
                DownloadEvent {
                    model_id: id.clone(),
                    status: p.status,
                    downloaded_bytes: p.downloaded_bytes,
                    total_bytes: p.total_bytes,
                    speed_bps: p.speed_bps,
                    eta_secs: p.eta_secs,
                    error: None,
                },
            )
        })
        .await;

        let result = result.and_then(|outcome| finish(&model, &dir, outcome).map(|_| outcome));
        let downloaded = task::bytes_on_disk(&model, &dir);
        let event = match result {
            Ok(Outcome::Done) => simple_event(&model, DownloadStatus::Done, model.total_size_bytes(), None),
            Ok(Outcome::Paused) => simple_event(&model, DownloadStatus::Paused, downloaded, None),
            Ok(Outcome::Cancelled) => simple_event(&model, DownloadStatus::Cancelled, 0, None),
            Err(e) => {
                eprintln!("[download] {}: {e:?}", model.id);
                simple_event(&model, DownloadStatus::Failed, downloaded, Some(e.to_string()))
            }
        };
        app.state::<DownloadManager>().release(&model.id);
        emit(&app, event);
    });
    Ok(())
}

/// Checks disk space and writes `download.json` before the first byte.
fn prepare(model: &CatalogModel, dir: &Path) -> AppResult<()> {
    let still_needed = model.total_size_bytes().saturating_sub(task::bytes_on_disk(model, dir));
    if let Some(space) = disk::space_for(dir) {
        let need = still_needed + config::DISK_SPACE_MARGIN_BYTES;
        if need > space.free_bytes {
            return Err(AppError::NotEnoughDisk { need, free: space.free_bytes });
        }
    }
    std::fs::create_dir_all(dir)?;
    if library::read_manifest(dir, PENDING_FILE).is_none() {
        library::write_manifest(dir, PENDING_FILE, &Manifest { model: model.clone(), saved_at: now_secs() })?;
    }
    Ok(())
}

/// After the task: mark the model as ready, or remove it after Cancel.
fn finish(model: &CatalogModel, dir: &Path, outcome: Outcome) -> AppResult<()> {
    match outcome {
        Outcome::Done => {
            library::write_manifest(dir, READY_FILE, &Manifest { model: model.clone(), saved_at: now_secs() })?;
            let _ = std::fs::remove_file(dir.join(PENDING_FILE));
        }
        Outcome::Cancelled => library::remove_model_dir(dir)?,
        Outcome::Paused => {}
    }
    Ok(())
}

#[tauri::command]
pub fn pause_download(model_id: String, manager: State<'_, DownloadManager>) {
    manager.signal(&model_id, Control::Pause);
}

/// Cancels a running download, or removes a paused one.
#[tauri::command]
pub fn cancel_download(
    app: AppHandle,
    model_id: String,
    manager: State<'_, DownloadManager>,
) -> AppResult<()> {
    if manager.cancel(&model_id) {
        return Ok(());
    }
    let dir = library::model_dir(&model_id)?;
    if library::is_ready(&dir) {
        return Ok(()); // Finished models are removed with Delete, not Cancel.
    }
    let model = library::read_manifest(&dir, PENDING_FILE).map(|m| m.model);
    library::remove_model_dir(&dir)?;
    if let Some(model) = model {
        emit(&app, simple_event(&model, DownloadStatus::Cancelled, 0, None));
    }
    Ok(())
}
