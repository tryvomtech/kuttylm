//! Search Hugging Face for GGUF models and list their downloadable versions.
//!
//! The versions a user looked at are kept here, so `start_download` can find
//! them by id. The frontend never sends file names or URLs.

mod api;
pub mod variants;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::catalog::types::CatalogModel;
use crate::error::AppResult;
use crate::fit::{fit_label, FitLabel};
use api::{SearchPage, SizeFilter, SortBy};

#[derive(Default)]
pub struct HfState(Mutex<HashMap<String, CatalogModel>>);

impl HfState {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, CatalogModel>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn find(&self, id: &str) -> Option<CatalogModel> {
        self.lock().get(id).cloned()
    }

    fn remember(&self, models: &[CatalogModel]) {
        let mut map = self.lock();
        for m in models {
            map.insert(m.id.clone(), m.clone());
        }
    }
}

#[tauri::command]
pub async fn hf_search(
    query: String,
    sort: SortBy,
    size: SizeFilter,
    cursor: Option<String>,
) -> AppResult<SearchPage> {
    api::search(&query, sort, size, cursor.as_deref()).await
}

#[derive(Debug, Serialize)]
pub struct HfVariant {
    #[serde(flatten)]
    pub model: CatalogModel,
    pub size_bytes: u64,
    pub fit: FitLabel,
    pub recommended: bool,
}

#[derive(Debug, Serialize)]
pub struct HfRepoDetail {
    pub repo: String,
    pub revision: String,
    pub params_b: Option<f64>,
    pub context_length: Option<u64>,
    pub license: Option<String>,
    pub vision: bool,
    pub variants: Vec<HfVariant>,
    /// Some GGUF files are in subfolders, which are not supported yet.
    pub has_folder_files: bool,
}

#[tauri::command]
pub async fn hf_repo_files(repo: String, state: State<'_, HfState>) -> AppResult<HfRepoDetail> {
    let (facts, tree) = api::repo_files(&repo).await?;
    let device = crate::device::detect_async().await?;
    let models = variants::build_variants(&facts, &tree);
    state.remember(&models);

    let fits = |m: &CatalogModel| fit_label(m.total_size_bytes(), &device) == FitLabel::FitsWell;
    let recommended = variants::recommend(&models, fits);
    let has_folder_files = tree
        .iter()
        .any(|e| e.kind == "file" && e.path.contains('/') && e.path.to_lowercase().ends_with(".gguf"));

    Ok(HfRepoDetail {
        vision: models.first().is_some_and(|m| m.mmproj.is_some()),
        repo: facts.repo,
        revision: facts.revision,
        params_b: facts.params_b,
        context_length: facts.context_length,
        license: facts.license,
        variants: models
            .into_iter()
            .enumerate()
            .map(|(i, model)| {
                let size_bytes = model.total_size_bytes();
                HfVariant { fit: fit_label(size_bytes, &device), size_bytes, recommended: Some(i) == recommended, model }
            })
            .collect(),
        has_folder_files,
    })
}

#[cfg(test)]
mod tests {
    /// Real calls to Hugging Face (a few KB). Run with:
    /// `cargo test hf::tests -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_search_and_files() {
        tauri::async_runtime::block_on(async {
            let page = super::api::search("smollm2 135m", super::SortBy::Downloads, super::SizeFilter::Tiny, None)
                .await
                .unwrap();
            println!("{} results, more: {}", page.results.len(), page.next_cursor.is_some());
            for r in page.results.iter().take(5) {
                println!("  {} ({} downloads, {:?})", r.id, r.downloads, r.license);
            }
            let (facts, tree) = super::api::repo_files("bartowski/SmolLM2-135M-Instruct-GGUF").await.unwrap();
            let v = super::variants::build_variants(&facts, &tree);
            println!("revision {}, ctx {:?}, params {:?}B, {} versions", facts.revision, facts.context_length, facts.params_b, v.len());
            for m in v.iter().take(4) {
                println!("  {} {} bytes", m.name, m.total_size_bytes());
            }
            assert!(!v.is_empty());
            // Gated models are refused with a clear message.
            let gated = super::api::repo_files("meta-llama/Llama-3.2-1B-Instruct").await;
            println!("gated repo: {:?}", gated.as_ref().err().map(|e| e.to_string()));
            assert!(gated.is_err());
        });
    }
}
