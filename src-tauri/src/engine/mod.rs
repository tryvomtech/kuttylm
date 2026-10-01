//! The model engine (llama-server). One model runs at a time.
//!
//! - Run: start llama-server hidden, wait for `/health`, then report `running`.
//! - "Auto" tries the graphics card first and falls back to the processor.
//! - Stop / app exit: SIGTERM, then SIGKILL after a few seconds.
//! - If the app crashes, the system kills llama-server (Linux: PR_SET_PDEATHSIG,
//!   Windows: a job object; macOS: not possible, only a normal exit stops it).

mod args;
pub(crate) mod logs;
pub(crate) mod process;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::process::Child;
use tokio::sync::watch;

use crate::catalog::types::CatalogModel;
use crate::error::{AppError, AppResult};
use crate::library::{self, READY_FILE};
use crate::platform;
use crate::settings::{self, GpuSetting, Settings};
use crate::util::now_secs;
use args::{build_args, gpu_mode_for, GpuMode, LaunchConfig};
use logs::LogBuffer;
use process::Wait;

pub const EVENT: &str = "engine-status";

/// Big models on slow disks can take a while to load.
const READY_TIMEOUT: Duration = Duration::from_secs(300);
const STOP_GRACE: Duration = Duration::from_secs(5);
const LOG_LINES_SHOWN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RunState {
    #[default]
    Stopped,
    Starting,
    Running,
    Failed,
}

/// Sent to the UI as the `engine-status` event.
#[derive(Debug, Clone, Default, Serialize)]
pub struct EngineStatus {
    pub state: RunState,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    /// OpenAI-compatible base URL, for example `http://127.0.0.1:41234/v1`.
    pub base_url: Option<String>,
    /// Key the app's own chat must send.
    pub api_key: Option<String>,
    /// `true` = graphics card allowed (separate card), `false` = processor only.
    pub uses_gpu: Option<bool>,
    /// Extra information, for example after falling back to the processor.
    pub note: Option<String>,
    pub error: Option<String>,
    /// Last lines of llama-server output when it failed.
    pub log_tail: Vec<String>,
    pub started_at: Option<u64>,
    /// Settings changed after the start. The model must restart to use them.
    pub needs_restart: bool,
    /// How other apps can reach the running model (set while starting or running).
    pub api: Option<ApiStatus>,
}

/// The API settings the running model was started with (not unsaved or newer settings).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ApiStatus {
    /// Other apps may use the model (fixed port). Off: only KuttyLM can.
    pub enabled: bool,
    pub port: Option<u16>,
    /// Other devices on the network can connect.
    pub lan: bool,
    /// Other apps must send the user's key.
    pub key_required: bool,
}

/// The settings that change how llama-server is started.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchKey {
    api_enabled: bool,
    api_port: u16,
    api_lan: bool,
    api_key: String,
    gpu: GpuSetting,
    gpu_layers: u32,
}

impl From<&Settings> for LaunchKey {
    fn from(s: &Settings) -> Self {
        Self {
            api_enabled: s.api_enabled,
            // Port, network and key only matter while the API is on.
            api_port: if s.api_enabled { s.api_port } else { 0 },
            api_lan: s.api_enabled && s.api_lan,
            api_key: if s.api_enabled { s.api_key.clone() } else { String::new() },
            gpu: s.gpu,
            gpu_layers: if s.gpu == GpuSetting::Custom { s.gpu_layers } else { 0 },
        }
    }
}

#[derive(Default)]
struct Inner {
    status: EngineStatus,
    /// The settings used for the current start.
    launched_with: Option<LaunchKey>,
    pid: Option<u32>,
    /// Becomes `true` when the llama-server process has ended.
    exited: Option<watch::Receiver<bool>>,
    /// Goes up on every start and stop. Old tasks see the change and give up.
    generation: u64,
}

#[derive(Default)]
pub struct Engine {
    inner: Mutex<Inner>,
    logs: LogBuffer,
}

impl Engine {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn status(&self) -> EngineStatus {
        let inner = self.lock();
        let mut status = inner.status.clone();
        let active = matches!(status.state, RunState::Starting | RunState::Running);
        status.needs_restart = active && inner.launched_with.as_ref() != Some(&LaunchKey::from(&settings::current()));
        status.api = inner.launched_with.as_ref().filter(|_| active).map(|k| ApiStatus {
            enabled: k.api_enabled,
            port: k.api_enabled.then_some(k.api_port),
            lan: k.api_lan,
            key_required: k.api_enabled && !k.api_key.is_empty(),
        });
        status
    }

    pub fn publish(&self, app: &AppHandle) {
        let _ = app.emit(EVENT, self.status());
    }

    /// Changes the status only if `generation` is still the current one.
    fn update(&self, app: &AppHandle, generation: u64, f: impl FnOnce(&mut Inner)) -> bool {
        {
            let mut inner = self.lock();
            if inner.generation != generation {
                return false;
            }
            f(&mut inner);
        }
        self.publish(app);
        true
    }

    fn fail(&self, app: &AppHandle, generation: u64, error: String) {
        eprintln!("[engine] could not start: {error}");
        let tail = self.logs.tail(LOG_LINES_SHOWN);
        self.update(app, generation, |inner| {
            inner.pid = None;
            inner.exited = None;
            let s = &mut inner.status;
            s.state = RunState::Failed;
            s.error = Some(error);
            s.log_tail = tail;
            s.base_url = None;
            s.api_key = None;
        });
    }

    /// Stops the running model (if any) and waits until the process has ended.
    pub async fn stop(&self, app: &AppHandle) {
        let (pid, exited) = {
            let mut inner = self.lock();
            inner.generation += 1;
            inner.status = EngineStatus::default();
            inner.launched_with = None;
            (inner.pid.take(), inner.exited.take())
        };
        self.publish(app);
        if let Some(pid) = pid {
            platform::terminate(pid);
            let ended = match exited {
                Some(mut rx) => tokio::time::timeout(STOP_GRACE, rx.wait_for(|done| *done)).await.is_ok(),
                None => false,
            };
            if !ended {
                platform::kill_now(pid);
            }
        }
    }

    /// Stops the model if it is `model_id`. Used before Delete.
    pub async fn stop_if(&self, app: &AppHandle, model_id: &str) {
        if self.status().model_id.as_deref() == Some(model_id) {
            self.stop(app).await;
        }
    }

    /// For app exit: runs outside async code, so it polls instead of awaiting.
    pub fn shutdown_blocking(&self) {
        let (pid, exited) = {
            let mut inner = self.lock();
            inner.generation += 1;
            // Not "running" any more, so the process watcher does not report a crash.
            inner.status = EngineStatus::default();
            inner.launched_with = None;
            (inner.pid.take(), inner.exited.take())
        };
        let Some(pid) = pid else { return };
        platform::terminate(pid);
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if exited.as_ref().is_some_and(|rx| *rx.borrow()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        platform::kill_now(pid);
    }
}

pub(crate) fn engine_exe(app: &AppHandle) -> AppResult<PathBuf> {
    let name = format!("llama/llama-server{}", std::env::consts::EXE_SUFFIX);
    if let Ok(dir) = app.path().resource_dir() {
        let exe = dir.join(&name);
        if exe.is_file() {
            return Ok(exe);
        }
    }
    // Dev builds: use the folder filled by the fetch script.
    let folder = if cfg!(windows) {
        "llama-windows"
    } else if cfg!(target_os = "macos") {
        "llama-macos"
    } else {
        "llama"
    };
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join(folder)
        .join(format!("llama-server{}", std::env::consts::EXE_SUFFIX));
    if cfg!(debug_assertions) && dev.is_file() {
        return Ok(dev);
    }
    Err(AppError::EngineMissing)
}

// ---------- commands ----------

#[tauri::command]
pub fn get_engine_status(engine: State<'_, Engine>) -> EngineStatus {
    engine.status()
}

#[tauri::command]
pub async fn start_model(app: AppHandle, model_id: String, engine: State<'_, Engine>) -> AppResult<()> {
    let dir = library::model_dir(&model_id)?;
    let manifest = library::read_manifest(&dir, READY_FILE).ok_or(AppError::ModelNotReady)?;
    if manifest.model.tags.iter().any(|t| t == "embedding") {
        return Err(AppError::NotAChatModel);
    }
    let exe = engine_exe(&app)?;
    let device = crate::device::detect_async().await?;

    // Only one model at a time.
    engine.stop(&app).await;

    let settings = settings::current();
    let generation = {
        let mut inner = engine.lock();
        inner.generation += 1;
        inner.launched_with = Some(LaunchKey::from(&settings));
        inner.status = EngineStatus {
            state: RunState::Starting,
            model_id: Some(manifest.model.id.clone()),
            model_name: Some(manifest.model.name.clone()),
            ..Default::default()
        };
        inner.generation
    };
    engine.publish(&app);

    let mode = gpu_mode_for(settings.gpu, settings.gpu_layers, &device);
    tauri::async_runtime::spawn(run(app.clone(), exe, dir, manifest.model, mode, settings, generation));
    Ok(())
}

#[tauri::command]
pub async fn stop_model(app: AppHandle, engine: State<'_, Engine>) -> AppResult<()> {
    engine.stop(&app).await;
    Ok(())
}

// ---------- start task ----------

/// Where llama-server listens and which key it needs.
struct Access {
    host: String,
    port: u16,
    api_key: Option<String>,
}

/// API off: this computer only, random port, hidden random key (only the app knows it).
/// API on: the port and key from Settings; all network addresses if LAN is on.
fn access_for(settings: &Settings) -> AppResult<Access> {
    if !settings.api_enabled {
        let port = process::free_port().map_err(|e| AppError::Internal(format!("no free port: {e}")))?;
        let key = crate::util::random_hex(24).map_err(|e| AppError::Internal(format!("no key: {e}")))?;
        return Ok(Access { host: "127.0.0.1".into(), port, api_key: Some(key) });
    }
    let host = if settings.api_lan { "0.0.0.0" } else { "127.0.0.1" };
    if !process::port_is_free(host, settings.api_port) {
        return Err(AppError::PortBusy(settings.api_port));
    }
    let key = (!settings.api_key.is_empty()).then(|| settings.api_key.clone());
    Ok(Access { host: host.into(), port: settings.api_port, api_key: key })
}

async fn run(
    app: AppHandle,
    exe: PathBuf,
    dir: PathBuf,
    model: CatalogModel,
    first: GpuMode,
    settings: Settings,
    generation: u64,
) {
    let engine = app.state::<Engine>();
    let Access { host, port, api_key } = match access_for(&settings) {
        Ok(a) => a,
        Err(e) => return engine.fail(&app, generation, e.to_string()),
    };
    // Any graphics card mode falls back to the processor if it fails.
    let modes: Vec<GpuMode> = if first == GpuMode::Off { vec![GpuMode::Off] } else { vec![first, GpuMode::Off] };

    for (attempt, &mode) in modes.iter().enumerate() {
        engine.logs.clear();
        let cfg = LaunchConfig {
            model_path: dir.join(&model.files[0].name),
            mmproj_path: model.mmproj.as_ref().map(|f| dir.join(&f.name)),
            host: host.clone(),
            port,
            api_key: api_key.clone(),
            model_context: model.context_length,
            gpu: mode,
        };
        let child = match process::spawn(&exe, &build_args(&cfg), &engine.logs) {
            Ok(c) => c,
            Err(e) => return engine.fail(&app, generation, format!("The model engine could not start: {e}")),
        };
        let Some(pid) = child.id() else {
            return engine.fail(&app, generation, "The model engine ended at once.".into());
        };
        let (exit_tx, exit_rx) = watch::channel(false);
        let attached = engine.update(&app, generation, |inner| {
            inner.pid = Some(pid);
            inner.exited = Some(exit_rx.clone());
        });
        tauri::async_runtime::spawn(watch_process(app.clone(), child, exit_tx, generation));
        if !attached {
            // Stopped while we were starting.
            platform::terminate(pid);
            return;
        }

        match process::wait_until_ready(port, exit_rx, READY_TIMEOUT).await {
            Wait::Ready => {
                let note = (attempt > 0).then(|| {
                    "The graphics card did not work, so the model runs on the processor. It may be slower.".to_string()
                });
                engine.update(&app, generation, |inner| {
                    let s = &mut inner.status;
                    s.state = RunState::Running;
                    s.base_url = Some(format!("http://127.0.0.1:{port}/v1"));
                    s.api_key = api_key.clone();
                    s.uses_gpu = Some(mode != GpuMode::Off);
                    s.note = note;
                    s.started_at = Some(now_secs());
                });
                return;
            }
            Wait::Exited if attempt + 1 < modes.len() => {
                eprintln!("[engine] start with graphics card failed; trying the processor only");
                continue;
            }
            Wait::Exited => {
                let reason = logs::explain_failure(&engine.logs.text()).to_string();
                return engine.fail(&app, generation, reason);
            }
            Wait::TimedOut => {
                platform::terminate(pid);
                return engine.fail(&app, generation, "The model took too long to start.".into());
            }
        }
    }
}

/// Waits for the process to end. If it ends while it should be running, report it.
async fn watch_process(app: AppHandle, mut child: Child, exit_tx: watch::Sender<bool>, generation: u64) {
    let result = child.wait().await;
    let _ = exit_tx.send(true);
    let engine = app.state::<Engine>();
    if engine.status().state == RunState::Running {
        let code = result.map(|s| s.to_string()).unwrap_or_default();
        eprintln!("[engine] llama-server ended unexpectedly: {code}");
        engine.fail(&app, generation, "The model stopped unexpectedly.".into());
    }
}

#[cfg(test)]
mod restart_tests {
    use super::LaunchKey;
    use crate::settings::{GpuSetting, Settings};

    #[test]
    fn api_status_follows_the_launch_settings() {
        let engine = super::Engine::default();
        assert_eq!(engine.status().api, None, "nothing runs");
        {
            let mut inner = engine.lock();
            inner.status.state = super::RunState::Running;
            inner.launched_with = Some(LaunchKey::from(&Settings {
                api_enabled: true,
                api_port: 9000,
                api_lan: true,
                api_key: "k".into(),
                ..Settings::default()
            }));
        }
        let api = engine.status().api.unwrap();
        assert!(api.enabled && api.lan && api.key_required);
        assert_eq!(api.port, Some(9000));
    }

    #[test]
    fn only_relevant_changes_need_a_restart() {
        let base = Settings::default();
        let key = |s: &Settings| LaunchKey::from(s);

        // API off: port and key are not used, so changing them needs no restart.
        let changed = Settings { api_port: 9000, api_key: "x".into(), ..base.clone() };
        assert_eq!(key(&base), key(&changed));
        // The models folder does not change a running model.
        let changed = Settings { models_dir: Some("/tmp/m".into()), ..base.clone() };
        assert_eq!(key(&base), key(&changed));

        // These do.
        assert_ne!(key(&base), key(&Settings { api_enabled: true, ..base.clone() }));
        assert_ne!(key(&base), key(&Settings { gpu: GpuSetting::Cpu, ..base.clone() }));
        let on = Settings { api_enabled: true, ..base.clone() };
        assert_ne!(key(&on), key(&Settings { api_key: "k".into(), ..on.clone() }));
        assert_ne!(key(&on), key(&Settings { api_lan: true, ..on.clone() }));
    }
}
