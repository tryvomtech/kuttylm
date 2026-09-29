//! Keeps the last good API response on disk, so the list still shows offline.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCatalog {
    /// Unix time (seconds) of the last successful fetch.
    pub fetched_at: u64,
    pub etag: Option<String>,
    /// The raw API response. It is parsed again on load with the same rules.
    pub body: String,
}

pub fn load() -> Option<CachedCatalog> {
    load_from(&paths::catalog_cache_file())
}

pub fn save(cache: &CachedCatalog) -> io::Result<()> {
    save_to(&paths::catalog_cache_file(), cache)
}

fn load_from(path: &Path) -> Option<CachedCatalog> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes to a temp file first, then renames, so a crash never leaves half a file.
fn save_to(path: &Path, cache: &CachedCatalog) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec(cache)?)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("kuttylm-cache-test-{}", std::process::id()));
        let path = dir.join("models.json");
        let cache = CachedCatalog {
            fetched_at: 1_790_000_000,
            etag: Some("\"abc\"".into()),
            body: r#"{"version":1,"models":[]}"#.into(),
        };
        save_to(&path, &cache).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.fetched_at, cache.fetched_at);
        assert_eq!(loaded.etag, cache.etag);
        assert_eq!(loaded.body, cache.body);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_or_broken_file_gives_none() {
        let dir = std::env::temp_dir().join(format!("kuttylm-cache-bad-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert!(load_from(&dir.join("nothing.json")).is_none());
        fs::write(dir.join("broken.json"), "{not json").unwrap();
        assert!(load_from(&dir.join("broken.json")).is_none());
        let _ = fs::remove_dir_all(dir);
    }
}
