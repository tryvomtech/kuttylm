//! "Chat with documents": the user adds PDF, Word, text or Markdown files.
//! Their text is cut into parts, and each part gets a vector from the document search model.
//! In a chat, the parts closest to the question are given to the chat model.

mod chunk;
pub mod embed;
mod parse;
mod search;
mod store;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, AppResult};
use embed::{Embedder, Purpose, EMBED_MODEL_ID};
use parse::DocKind;
use store::{DocMeta, DocStatus};

/// Bigger files are refused (a 50 MB PDF is several thousand pages).
const MAX_FILE_BYTES: usize = 50 * 1024 * 1024;
const MAX_NAME_CHARS: usize = 200;
const MAX_RESULTS: usize = 12;

/// Documents being indexed now, with a "stop" flag each.
#[derive(Default)]
pub struct DocsState {
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl DocsState {
    fn jobs(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.jobs.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn is_active(&self, id: &str) -> bool {
        self.jobs().contains_key(id)
    }
}

#[derive(Debug, Serialize)]
pub struct DocsStatus {
    /// The document search model is downloaded.
    pub model_ready: bool,
    pub model_id: &'static str,
    pub model_name: String,
    pub model_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
struct Progress {
    id: String,
    done: usize,
    total: usize,
    status: DocStatus,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub doc_id: String,
    pub doc_name: String,
    pub page: Option<u32>,
    pub text: String,
    pub score: f32,
}

#[tauri::command]
pub fn get_docs_status() -> DocsStatus {
    let model = embed::embed_model();
    DocsStatus {
        model_ready: embed::model_file().is_some(),
        model_id: EMBED_MODEL_ID,
        model_size_bytes: model.total_size_bytes(),
        model_name: model.name,
    }
}

#[tauri::command]
pub async fn list_documents(docs: State<'_, DocsState>) -> AppResult<Vec<DocMeta>> {
    let mut list = tauri::async_runtime::spawn_blocking(|| store::list(&store::root()))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    // "Indexing" without a running job: the app was closed while it worked.
    for doc in &mut list {
        if doc.status == DocStatus::Indexing && !docs.is_active(&doc.id) {
            doc.status = DocStatus::Failed;
            doc.error = Some("KuttyLM was closed before this document was ready. Delete it and add it again.".into());
        }
    }
    Ok(list)
}

/// The file comes as raw bytes; its name is in the `x-file-name` header (URL-encoded).
#[tauri::command]
pub async fn add_document(app: AppHandle, request: Request<'_>, docs: State<'_, DocsState>) -> AppResult<DocMeta> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::Internal("expected the file as raw bytes".into()));
    };
    let name = request
        .headers()
        .get("x-file-name")
        .and_then(|v| v.to_str().ok())
        .map(percent_decode)
        .map(|n| clean_name(&n))
        .filter(|n| !n.is_empty())
        .ok_or_else(|| AppError::Internal("missing file name".into()))?;
    let kind = DocKind::from_name(&name).ok_or_else(|| {
        AppError::Document("Only PDF, Word (.docx), text (.txt) and Markdown (.md) files can be added.".into())
    })?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(AppError::Document(format!(
            "This file is too big ({}). The limit is {}.",
            crate::format::human_bytes(bytes.len() as u64),
            crate::format::human_bytes(MAX_FILE_BYTES as u64)
        )));
    }
    if bytes.is_empty() {
        return Err(AppError::Document("This file is empty.".into()));
    }
    if embed::model_file().is_none() {
        return Err(AppError::EmbedModelMissing);
    }

    let id = format!(
        "{}-{}",
        crate::util::now_secs(),
        crate::util::random_hex(4).map_err(|e| AppError::Internal(e.to_string()))?
    );
    let root = store::root();
    let dir = store::doc_dir(&root, &id)?;
    std::fs::create_dir_all(&dir)?;
    let meta = DocMeta {
        id: id.clone(),
        name,
        kind,
        size_bytes: bytes.len() as u64,
        pages: None,
        chunks: 0,
        created_at: crate::util::now_secs(),
        status: DocStatus::Indexing,
        error: None,
        model_id: Some(EMBED_MODEL_ID.into()),
        dim: 0,
    };
    store::write_meta(&dir, &meta)?;

    let stop = Arc::new(AtomicBool::new(false));
    docs.jobs().insert(id.clone(), stop.clone());
    let bytes = bytes.clone();
    let job_meta = meta.clone();
    tauri::async_runtime::spawn(async move {
        let result = index(&app, &dir, job_meta.clone(), bytes, &stop).await;
        if let Err(e) = result {
            if !stop.load(Ordering::Relaxed) {
                let failed = DocMeta { status: DocStatus::Failed, error: Some(e.to_string()), ..job_meta };
                let _ = store::write_meta(&dir, &failed);
                emit(&app, Progress { id: failed.id.clone(), done: 0, total: 0, status: DocStatus::Failed, error: failed.error });
            }
        }
        app.state::<DocsState>().jobs().remove(&id);
    });
    Ok(meta)
}

async fn index(app: &AppHandle, dir: &Path, mut meta: DocMeta, bytes: Vec<u8>, stop: &AtomicBool) -> AppResult<()> {
    let kind = meta.kind;
    let (chunks, pages) = tauri::async_runtime::spawn_blocking(move || {
        let sections = parse::extract(kind, &bytes)?;
        let pages = (kind == DocKind::Pdf).then_some(sections.len() as u32);
        Ok::<_, String>((chunk::chunk_sections(&sections), pages))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .map_err(AppError::Document)?;

    let total = chunks.len();
    meta.pages = pages;
    meta.chunks = total;
    store::write_chunks(dir, &chunks)?;
    store::write_meta(dir, &meta)?;

    let embedder = app.state::<Embedder>();
    let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(total);
    emit(app, Progress { id: meta.id.clone(), done: 0, total, status: DocStatus::Indexing, error: None });
    for batch in chunks.chunks(embed::BATCH) {
        if stop.load(Ordering::Relaxed) {
            return Err(AppError::Document("stopped".into()));
        }
        let texts: Vec<String> = batch.iter().map(|c| c.text.clone()).collect();
        vectors.extend(embedder.embed(app, Purpose::Document, &texts).await?);
        emit(app, Progress { id: meta.id.clone(), done: vectors.len(), total, status: DocStatus::Indexing, error: None });
    }
    if stop.load(Ordering::Relaxed) {
        return Err(AppError::Document("stopped".into()));
    }
    let dim = vectors.first().map_or(0, Vec::len);
    if dim == 0 || vectors.iter().any(|v| v.len() != dim) {
        return Err(AppError::Document("Document search failed: the model gave no usable vectors.".into()));
    }
    store::write_vectors(dir, &vectors)?;
    meta.dim = dim;
    meta.status = DocStatus::Ready;
    store::write_meta(dir, &meta)?;
    emit(app, Progress { id: meta.id, done: total, total, status: DocStatus::Ready, error: None });
    Ok(())
}

fn emit(app: &AppHandle, progress: Progress) {
    let _ = app.emit("document-progress", progress);
}

#[tauri::command]
pub async fn delete_document(id: String, docs: State<'_, DocsState>) -> AppResult<()> {
    if let Some(stop) = docs.jobs().remove(&id) {
        stop.store(true, Ordering::Relaxed);
    }
    store::remove(&store::root(), &id)
}

/// The parts of the chosen documents that best match `query`, best first.
#[tauri::command]
pub async fn search_documents(app: AppHandle, ids: Vec<String>, query: String, k: usize) -> AppResult<Vec<SearchHit>> {
    let k = k.clamp(1, MAX_RESULTS);
    let root = store::root();
    let ids: Vec<String> = ids.into_iter().filter(|id| store::doc_dir(&root, id).is_ok()).collect();
    if ids.is_empty() || query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let query_vec = app
        .state::<Embedder>()
        .embed(&app, Purpose::Query, &[query])
        .await?
        .pop()
        .unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || search_in(&root, &ids, &query_vec, k))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

fn search_in(root: &Path, ids: &[String], query: &[f32], k: usize) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    for id in ids {
        let Ok(dir) = store::doc_dir(root, id) else { continue };
        let Some(meta) = store::read_meta(&dir) else { continue };
        if meta.status != DocStatus::Ready || meta.model_id.as_deref() != Some(EMBED_MODEL_ID) {
            continue;
        }
        let (Some(chunks), Some(vectors)) = (store::read_chunks(&dir), store::read_vectors(&dir)) else {
            continue;
        };
        if vectors.len() != chunks.len() * meta.dim {
            continue;
        }
        for (i, score) in search::top_k(query, &vectors, meta.dim, k) {
            let c = &chunks[i];
            hits.push(SearchHit {
                doc_id: meta.id.clone(),
                doc_name: meta.name.clone(),
                page: c.page,
                text: c.text.clone(),
                score,
            });
        }
    }
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits.truncate(k);
    hits
}

/// Stops the search model before its files are deleted.
pub async fn before_model_delete(app: &AppHandle, model_id: &str) {
    if model_id == EMBED_MODEL_ID {
        app.state::<Embedder>().stop().await;
    }
}

/// Only the file name (no folders), without control characters, not too long.
fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect::<String>().trim().to_string()
}

/// Decodes `%XX` (from `encodeURIComponent`). Bad sequences are kept as they are.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(percent_decode("My%20Report%20%E0%AE%A4.pdf"), "My Report த.pdf");
        assert_eq!(percent_decode("100%.txt"), "100%.txt");
        assert_eq!(percent_decode("a%zz.md"), "a%zz.md");
        assert_eq!(clean_name("C:\\Users\\me\\notes.md"), "notes.md");
        assert_eq!(clean_name("../../etc/x.txt"), "x.txt");
    }
}
