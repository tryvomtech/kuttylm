//! The model list: from the API (with an offline cache) or from the built-in mock.

mod cache;
mod fetch;
mod parse;
pub mod types;

use std::sync::Mutex;

use serde::Serialize;

use crate::device::{self, DeviceInfo};
use crate::error::{AppError, AppResult};
use crate::fit::{fit_label, FitLabel};
use cache::CachedCatalog;
use fetch::FetchOutcome;
use parse::{parse, ParseError, ParsedCatalog};
use crate::util::now_secs;
use types::CatalogModel;

const MOCK_BODY: &str = include_str!("../../resources/models.mock.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ListSource {
    /// Fresh from the API.
    Api,
    /// The API failed; this is the last saved list.
    Cache,
    /// No API URL is set; this is the built-in sample list.
    Mock,
}

#[derive(Debug, Serialize)]
pub struct ModelListItem {
    #[serde(flatten)]
    pub model: CatalogModel,
    pub size_bytes: u64,
    pub fit: FitLabel,
}

#[derive(Debug, Serialize)]
pub struct ModelList {
    pub models: Vec<ModelListItem>,
    pub source: ListSource,
    /// Unix seconds of the fetch this list comes from (not set for the mock).
    pub fetched_at: Option<u64>,
    /// The API sent a newer format. The user should update the app.
    pub needs_update: bool,
    /// Broken entries that were left out.
    pub skipped: usize,
}

/// The last loaded list. Downloads look up models here by id,
/// so the frontend never sends file names or URLs.
#[derive(Default)]
pub struct CatalogState(Mutex<Vec<CatalogModel>>);

impl CatalogState {
    pub fn find(&self, id: &str) -> Option<CatalogModel> {
        let models = self.0.lock().unwrap_or_else(|e| e.into_inner());
        models.iter().find(|m| m.id == id).cloned()
    }

    fn remember(&self, list: &ModelList) {
        let mut models = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *models = list.models.iter().map(|m| m.model.clone()).collect();
    }
}

#[tauri::command]
pub async fn get_models(state: tauri::State<'_, CatalogState>) -> AppResult<ModelList> {
    let list = load_list().await?;
    state.remember(&list);
    Ok(list)
}

async fn load_list() -> AppResult<ModelList> {
    let device = device::detect_async().await?;

    let url = crate::config::models_api_url();
    let loaded = if url.is_empty() {
        let parsed = parse(MOCK_BODY)
            .map_err(|e| AppError::Internal(format!("built-in list is broken: {e:?}")))?;
        Loaded { parsed, source: ListSource::Mock, fetched_at: None, needs_update: false }
    } else {
        load_from_api(&url).await?
    };

    Ok(build_list(loaded, &device))
}

struct Loaded {
    parsed: ParsedCatalog,
    source: ListSource,
    fetched_at: Option<u64>,
    needs_update: bool,
}

async fn load_from_api(url: &str) -> AppResult<Loaded> {
    let cached = cache::load();
    let etag = cached.as_ref().and_then(|c| c.etag.as_deref());
    let mut needs_update = false;

    match fetch::fetch(url, etag).await {
        Ok(FetchOutcome::Fresh { body, etag }) => match parse(&body) {
            Ok(parsed) => {
                let fetched_at = now_secs();
                if let Err(e) = cache::save(&CachedCatalog { fetched_at, etag, body }) {
                    eprintln!("[catalog] could not save cache: {e}");
                }
                return Ok(Loaded {
                    parsed,
                    source: ListSource::Api,
                    fetched_at: Some(fetched_at),
                    needs_update: false,
                });
            }
            Err(ParseError::TooNew(v)) => {
                eprintln!("[catalog] API sends version {v}; app knows {}", crate::config::MODELS_API_VERSION);
                needs_update = true;
            }
            Err(e) => eprintln!("[catalog] API response not usable: {e:?}"),
        },
        Ok(FetchOutcome::NotModified) => {
            if let Some(mut c) = cached.clone() {
                if let Ok(parsed) = parse(&c.body) {
                    c.fetched_at = now_secs();
                    let _ = cache::save(&c);
                    return Ok(Loaded {
                        parsed,
                        source: ListSource::Api,
                        fetched_at: Some(c.fetched_at),
                        needs_update: false,
                    });
                }
            }
        }
        Err(e) => eprintln!("[catalog] fetch failed: {e}"),
    }

    // The API did not give a usable list. Show the last saved one.
    if let Some(c) = cached {
        if let Ok(parsed) = parse(&c.body) {
            return Ok(Loaded {
                parsed,
                source: ListSource::Cache,
                fetched_at: Some(c.fetched_at),
                needs_update,
            });
        }
    }
    Err(if needs_update { AppError::AppTooOld } else { AppError::ModelListUnavailable })
}

fn build_list(loaded: Loaded, device: &DeviceInfo) -> ModelList {
    let mut models: Vec<ModelListItem> = loaded
        .parsed
        .models
        .into_iter()
        .map(|model| {
            let size_bytes = model.total_size_bytes();
            ModelListItem { fit: fit_label(size_bytes, device), size_bytes, model }
        })
        .collect();
    // Trending first; keep the API order otherwise.
    models.sort_by_key(|m| !m.model.trending);

    ModelList {
        models,
        source: loaded.source,
        fetched_at: loaded.fetched_at,
        needs_update: loaded.needs_update,
        skipped: loaded.parsed.skipped,
    }
}


#[cfg(test)]
pub fn parse_for_tests(body: &str) -> Vec<CatalogModel> {
    parse(body).expect("valid list").models
}

#[cfg(test)]
mod tests {
    /// Prints the list as the app would get it on this computer. Run with:
    /// `cargo test print_model_list -- --ignored --nocapture`
    /// Set `KUTTYLM_MODELS_API_URL` to test a real or local API.
    #[test]
    #[ignore]
    fn print_model_list() {
        match tauri::async_runtime::block_on(super::load_list()) {
            Ok(list) => {
                println!(
                    "source={:?} fetched_at={:?} needs_update={} skipped={}",
                    list.source, list.fetched_at, list.needs_update, list.skipped
                );
                for m in list.models {
                    println!("{:<28} {:>6.2} GB  {:?}", m.model.id, m.size_bytes as f64 / 1e9, m.fit);
                }
            }
            Err(e) => println!("error: {e}"),
        }
    }
}
