//! All folders the app uses, in one place.
//! Linux:   ~/.cache/kuttylm, ~/.local/share/kuttylm, ~/.config/kuttylm
//! Windows: %LOCALAPPDATA%\kuttylm (cache and models), %APPDATA%\kuttylm (settings)
//! macOS:   ~/Library/Caches/kuttylm, ~/Library/Application Support/kuttylm

use std::path::{Path, PathBuf};

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

/// For the UI: `/home/anna/.local/share/x` → `~/.local/share/x` (Linux and macOS).
/// Shorter, and it does not show the user name.
pub fn display(path: &Path) -> String {
    if cfg!(unix) {
        if let Some(rest) = dirs::home_dir().and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf)) {
            return if rest.as_os_str().is_empty() { "~".into() } else { format!("~/{}", rest.display()) };
        }
    }
    path.to_string_lossy().into_owned()
}

/// The reverse of `display`: `~/models` → `/home/anna/models`. Other paths stay as they are.
pub fn expand_home(text: &str) -> PathBuf {
    match (text.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) if cfg!(unix) => home.join(rest),
        _ if text == "~" && cfg!(unix) => dirs::home_dir().unwrap_or_else(|| PathBuf::from(text)),
        _ => PathBuf::from(text),
    }
}

/// Documents for "chat with documents": text parts and their search vectors.
pub fn documents_dir() -> PathBuf {
    data_dir().join("documents")
}

/// Saved chats, one file per chat.
pub fn chats_dir() -> PathBuf {
    data_dir().join("chats")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn home_is_shown_as_tilde() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(display(&home.join(".local/share/kuttylm")), "~/.local/share/kuttylm");
        assert_eq!(display(&home), "~");
        assert_eq!(display(Path::new("/opt/models")), "/opt/models");
        assert_eq!(expand_home("~/models"), home.join("models"));
        assert_eq!(expand_home("/opt/models"), PathBuf::from("/opt/models"));
    }
}
