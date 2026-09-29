//! Build-time settings that a developer may change.

/// URL of the "Picked for you" model list: `catalog/models.json` in the GitHub repository.
/// Edit that file to change the list for every user, with no new app release.
/// Empty → the app uses only the list built into it.
///
/// For testing, the environment variable `KUTTYLM_MODELS_API_URL` overrides it.
pub const MODELS_API_URL: &str = "https://raw.githubusercontent.com/tryvomtech/kuttylm/main/catalog/models.json";

/// How long to wait for the model list API.
pub const MODELS_API_TIMEOUT_SECS: u64 = 10;

/// The highest model list format version this app understands.
pub const MODELS_API_VERSION: u64 = 1;

pub fn models_api_url() -> String {
    std::env::var("KUTTYLM_MODELS_API_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| MODELS_API_URL.to_string())
}

pub fn user_agent() -> String {
    format!("KuttyLM/{}", env!("CARGO_PKG_VERSION"))
}

/// Where model files are downloaded from:
/// `{HF_BASE_URL}/{hf_repo}/resolve/{revision}/{file}`.
pub const HF_BASE_URL: &str = "https://huggingface.co";

/// Extra free disk space to keep after a download.
pub const DISK_SPACE_MARGIN_BYTES: u64 = 500 * 1024 * 1024;

/// `KUTTYLM_HF_BASE_URL` overrides the download server (for tests with a local server).
pub fn hf_base_url() -> String {
    std::env::var("KUTTYLM_HF_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| HF_BASE_URL.to_string())
}
