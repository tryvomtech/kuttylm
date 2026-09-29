//! All folders the app uses, in one place.
//! Linux:   ~/.cache/kuttylm, ~/.local/share/kuttylm, ~/.config/kuttylm
//! Windows: %LOCALAPPDATA%\kuttylm (cache and models), %APPDATA%\kuttylm (settings)
//! macOS:   ~/Library/Caches/kuttylm, ~/Library/Application Support/kuttylm

use std::path::PathBuf;

const APP_DIR: &str = "kuttylm";

fn base(dir: Option<PathBuf>) -> PathBuf {
    dir.unwrap_or_else(std::env::temp_dir).join(APP_DIR)
}

pub fn cache_dir() -> PathBuf {
    base(dirs::cache_dir())
}

/// Local (not roaming) data: models are too big to sync with a Windows profile.
pub fn data_dir() -> PathBuf {
    base(dirs::data_local_dir())
}

pub fn config_dir() -> PathBuf {
    base(dirs::config_dir())
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn catalog_cache_file() -> PathBuf {
    cache_dir().join("models.json")
}

pub fn default_models_dir() -> PathBuf {
    data_dir().join("models")
}

/// One folder per model inside: `models/<model-id>/`.
/// Order: `KUTTYLM_MODELS_DIR` (for tests), then Settings, then the default.
pub fn models_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("KUTTYLM_MODELS_DIR") {
        return PathBuf::from(dir);
    }
    crate::settings::current()
        .models_dir
        .map(PathBuf::from)
        .unwrap_or_else(default_models_dir)
}
