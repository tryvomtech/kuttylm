//! User settings, saved in `~/.config/kuttylm/settings.json`.

use std::fs;
use std::path::Path;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::download::DownloadManager;
use crate::engine::Engine;
use crate::error::{AppError, AppResult};
use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GpuSetting {
    /// Graphics card only when there is a separate one (recommended).
    #[default]
    Auto,
    /// Always try the graphics card, also a built-in one.
    Gpu,
    /// Processor only.
    Cpu,
    /// A fixed number of layers on the graphics card.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Other apps may use the model through the OpenAI-compatible API.
    pub api_enabled: bool,
    pub api_port: u16,
    /// Listen on all network addresses, not only this computer.
    pub api_lan: bool,
    /// Empty = no key needed.
    pub api_key: String,
    /// `None` = the default folder.
    pub models_dir: Option<String>,
    pub gpu: GpuSetting,
    /// Used when `gpu` is `Custom`.
    pub gpu_layers: u32,
    /// Keep chats on this computer, so they can be opened again later.
    pub save_chats: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_enabled: false,
            api_port: 8080,
            api_lan: false,
            api_key: String::new(),
            models_dir: None,
            gpu: GpuSetting::Auto,
            gpu_layers: 20,
            save_chats: true,
        }
    }
}

impl Settings {
    /// Checks and cleans the values. Returns a friendly error for the UI.
    pub fn validated(mut self) -> AppResult<Self> {
        let bad = |msg: &str| Err(AppError::InvalidSetting(msg.to_string()));
        if self.api_port < 1024 {
            return bad("The port must be a number from 1024 to 65535.");
        }
        self.api_key = self.api_key.trim().to_string();
        if self.api_key.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return bad("The API key cannot contain spaces.");
        }
        if self.api_key.len() > 200 {
            return bad("The API key is too long (200 characters at most).");
        }
        if self.gpu_layers > 999 {
            return bad("The number of graphics card layers must be from 0 to 999.");
        }
        self.models_dir = match self.models_dir.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(dir) => {
                let path = paths::expand_home(dir);
                if !path.is_absolute() {
                    return bad("The models folder must be a full path, for example /home/me/models.");
                }
                if path == paths::default_models_dir() {
                    None
                } else {
                    Some(path.to_string_lossy().into_owned())
                }
            }
        };
        Ok(self)
    }
}

// ---------- the current settings, for all modules ----------

static CURRENT: RwLock<Option<Settings>> = RwLock::new(None);

pub fn current() -> Settings {
    if let Some(s) = CURRENT.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return s.clone();
    }
    let loaded = load_from(&paths::settings_file());
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = Some(loaded.clone());
    loaded
}

fn load_from(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .and_then(|s| s.validated().ok())
        .unwrap_or_default()
}

fn save_to(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(tmp, path)
}

/// A folder is usable if it can be created and written to.
fn check_folder(dir: &Path) -> AppResult<()> {
    fs::create_dir_all(dir)
        .and_then(|_| {
            let probe = dir.join(".kuttylm-write-test");
            fs::write(&probe, b"ok")?;
            fs::remove_file(probe)
        })
        .map_err(|e| AppError::InvalidSetting(format!("KuttyLM cannot save files in this folder: {e}")))
}

// ---------- commands ----------

#[tauri::command]
pub fn get_settings() -> Settings {
    current()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    settings: Settings,
    downloads: State<'_, DownloadManager>,
    engine: State<'_, Engine>,
) -> AppResult<Settings> {
    let new = settings.validated()?;
    let old = current();
    if new.models_dir != old.models_dir {
        if downloads.is_busy() {
            return Err(AppError::InvalidSetting(
                "Please wait until the download finishes (or pause it) before you change the models folder.".into(),
            ));
        }
        if let Some(dir) = &new.models_dir {
            check_folder(Path::new(dir))?;
        }
    }
    save_to(&paths::settings_file(), &new)?;
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = Some(new.clone());
    // The status says if the running model needs a restart for the new settings.
    engine.publish(&app);
    Ok(new)
}

#[derive(Debug, Serialize)]
pub struct AppPaths {
    /// Shown as the example in the models folder field.
    pub default_models_dir: String,
}

#[tauri::command]
pub fn get_app_paths() -> AppPaths {
    AppPaths { default_models_dir: paths::display(&paths::default_models_dir()) }
}

#[derive(Debug, Serialize)]
pub struct NetworkInfo {
    /// This computer's address on the local network, if it has one.
    pub lan_ip: Option<String>,
}

#[tauri::command]
pub fn get_network_info() -> NetworkInfo {
    NetworkInfo { lan_ip: lan_ip() }
}

/// Asks the system which local address it would use to reach the internet.
/// A UDP "connect" sends no packet; it only picks the route.
fn lan_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?; // TEST-NET address, never contacted
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let s = Settings::default();
        assert!(!s.api_enabled);
        assert_eq!(s.api_port, 8080);
        assert!(!s.api_lan);
        assert!(s.api_key.is_empty());
        assert_eq!(s.gpu, GpuSetting::Auto);
    }

    #[test]
    fn validation() {
        let ok = |s: Settings| s.validated().is_ok();
        assert!(ok(Settings::default()));
        assert!(!ok(Settings { api_port: 80, ..Default::default() }));
        assert!(!ok(Settings { api_key: "has space".into(), ..Default::default() }));
        assert!(!ok(Settings { models_dir: Some("relative/dir".into()), ..Default::default() }));
        assert!(!ok(Settings { gpu_layers: 5000, ..Default::default() }));

        let cleaned = Settings { api_key: "  key1  ".into(), models_dir: Some("  ".into()), ..Default::default() }
            .validated()
            .unwrap();
        assert_eq!(cleaned.api_key, "key1");
        assert_eq!(cleaned.models_dir, None);
    }

    #[test]
    fn old_or_partial_file_uses_defaults_for_missing_fields() {
        let dir = std::env::temp_dir().join(format!("kuttylm-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, r#"{ "api_enabled": true, "future_field": 1 }"#).unwrap();
        let s = load_from(&path);
        assert!(s.api_enabled);
        assert_eq!(s.api_port, 8080);

        fs::write(&path, "{ broken").unwrap();
        assert_eq!(load_from(&path), Settings::default());

        let custom = Settings { api_port: 9000, ..Default::default() };
        save_to(&path, &custom).unwrap();
        assert_eq!(load_from(&path), custom);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lan_ip_is_not_loopback() {
        if let Some(ip) = lan_ip() {
            assert!(!ip.starts_with("127."));
        }
    }
}
