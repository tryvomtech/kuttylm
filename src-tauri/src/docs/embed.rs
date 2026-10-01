//! The document search model: a small embedding model run by a second llama-server.
//!
//! It is not bundled. The user downloads it once (it is listed in My models).
//! The server starts when it is needed and stops after some idle time,
//! so it uses no memory while documents are not used.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::AppHandle;
use tokio::process::Child;
use tokio::sync::watch;

use crate::catalog::types::{CatalogFile, CatalogModel};
use crate::engine::logs::{self, LogBuffer};
use crate::engine::process::{self, Wait};
use crate::error::{AppError, AppResult};
use crate::library;
use crate::platform;

pub const EMBED_MODEL_ID: &str = "nomic-embed-text-v1.5-q4km";
/// Tag that marks a model as "search only" (no Run button, no chat).
pub const EMBED_TAG: &str = "embedding";

/// Texts per request while a document is indexed.
pub const BATCH: usize = 16;
/// The model reads at most 2048 tokens. Longer search questions are cut.
const MAX_QUERY_CHARS: usize = 2000;
const IDLE_STOP: Duration = Duration::from_secs(5 * 60);
const START_TIMEOUT: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// nomic-embed-text v1.5 (137M parameters, Q4_K_M, 84 MB). Apache-2.0.
pub fn embed_model() -> CatalogModel {
    CatalogModel {
        id: EMBED_MODEL_ID.into(),
        name: "Document search model".into(),
        description: Some(
            "Needed for Documents. It finds the parts of your documents that answer a question. It cannot chat."
                .into(),
        ),
        hf_repo: "nomic-ai/nomic-embed-text-v1.5-GGUF".into(),
        revision: "0188c9bf409793f810680a5a431e7b899c46104c".into(),
        files: vec![CatalogFile {
            name: "nomic-embed-text-v1.5.Q4_K_M.gguf".into(),
            size_bytes: 84_106_624,
            sha256: Some("d4e388894e09cf3816e8b0896d81d265b55e7a9fff9ab03fe8bf4ef5e11295ac".into()),
        }],
        mmproj: None,
        quantization: Some("Q4_K_M".into()),
        params_b: Some(0.137),
        context_length: Some(2048),
        license: Some("apache-2.0".into()),
        tags: vec![EMBED_TAG.into()],
        trending: false,
    }
}

/// The model file, if the model is fully downloaded.
pub fn model_file() -> Option<PathBuf> {
    let dir = library::model_dir(EMBED_MODEL_ID).ok()?;
    let manifest = library::read_manifest(&dir, library::READY_FILE)?;
    let path = dir.join(&manifest.model.files.first()?.name);
    path.is_file().then_some(path)
}

/// nomic needs a task prefix in front of each text.
pub enum Purpose {
    Document,
    Query,
}

fn with_prefix(purpose: &Purpose, text: &str) -> String {
    match purpose {
        Purpose::Document => format!("search_document: {text}"),
        Purpose::Query => {
            let cut: String = text.chars().take(MAX_QUERY_CHARS).collect();
            format!("search_query: {cut}")
        }
    }
}

struct Server {
    pid: u32,
    port: u16,
    key: String,
    exited: watch::Receiver<bool>,
}

#[derive(Default)]
pub struct Embedder {
    /// Only one start or request at a time.
    gate: tokio::sync::Mutex<()>,
    server: Mutex<Option<Server>>,
    last_used: Mutex<Option<Instant>>,
    logs: LogBuffer,
}

impl Embedder {
    fn server(&self) -> MutexGuard<'_, Option<Server>> {
        self.server.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn touch(&self) {
        *self.last_used.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
    }

    /// Vectors (length 1, same order as `texts`).
    pub async fn embed(&self, app: &AppHandle, purpose: Purpose, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let _gate = self.gate.lock().await;
        let (port, key) = self.ensure_running(app).await?;
        self.touch();
        let input: Vec<String> = texts.iter().map(|t| with_prefix(&purpose, t)).collect();
        let result = request(port, &key, &input).await;
        self.touch();
        result
    }

    async fn ensure_running(&self, app: &AppHandle) -> AppResult<(u16, String)> {
        {
            let mut server = self.server();
            match &*server {
                Some(s) if !*s.exited.borrow() => return Ok((s.port, s.key.clone())),
                Some(_) => *server = None,
                None => {}
            }
        }
        let model = model_file().ok_or(AppError::EmbedModelMissing)?;
        let exe = crate::engine::engine_exe(app)?;
        let port = process::free_port().map_err(|e| AppError::Internal(format!("no free port: {e}")))?;
        let key = crate::util::random_hex(24).map_err(|e| AppError::Internal(format!("no key: {e}")))?;

        self.logs.clear();
        let child = process::spawn(&exe, &server_args(&model, port, &key), &self.logs)
            .map_err(|e| AppError::Document(format!("The document search model could not start: {e}")))?;
        let pid = child.id().ok_or_else(|| AppError::Document("The document search model ended at once.".into()))?;
        let (exit_tx, exit_rx) = watch::channel(false);
        tauri::async_runtime::spawn(wait_for_exit(child, exit_tx));

        match process::wait_until_ready(port, exit_rx.clone(), START_TIMEOUT).await {
            Wait::Ready => {}
            Wait::Exited => {
                let reason = logs::explain_failure(&self.logs.text());
                return Err(AppError::Document(format!("The document search model could not start. {reason}")));
            }
            Wait::TimedOut => {
                platform::terminate(pid);
                return Err(AppError::Document("The document search model took too long to start.".into()));
            }
        }
        let first_start = self.last_used.lock().unwrap_or_else(|e| e.into_inner()).is_none();
        *self.server() = Some(Server { pid, port, key: key.clone(), exited: exit_rx });
        self.touch();
        if first_start {
            spawn_idle_watch(app.clone());
        }
        Ok((port, key))
    }

    /// Stops the server (for example before its model is deleted).
    pub async fn stop(&self) {
        let _gate = self.gate.lock().await;
        self.stop_now();
    }

    fn stop_now(&self) {
        if let Some(s) = self.server().take() {
            platform::terminate(s.pid);
        }
    }

    /// For app exit: no async code here.
    pub fn shutdown_blocking(&self) {
        let Some(s) = self.server().take() else { return };
        platform::terminate(s.pid);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if *s.exited.borrow() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        platform::kill_now(s.pid);
    }
}

fn server_args(model: &std::path::Path, port: u16, key: &str) -> Vec<String> {
    [
        "--model",
        &model.to_string_lossy(),
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "--api-key",
        key,
        "--embedding",
        // One text at a time, up to the model's full length.
        "--ctx-size",
        "2048",
        "--batch-size",
        "2048",
        "--ubatch-size",
        "2048",
        "--parallel",
        "1",
        // The model is small; the processor is fast enough and always works.
        "--device",
        "none",
        "--no-webui",
        "--offline",
        "--log-colors",
        "off",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

async fn wait_for_exit(mut child: Child, exit_tx: watch::Sender<bool>) {
    let _ = child.wait().await;
    let _ = exit_tx.send(true);
}

/// Stops the server after `IDLE_STOP` without use. Runs for the life of the app.
fn spawn_idle_watch(app: AppHandle) {
    use tauri::Manager;
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let embedder = app.state::<Embedder>();
            let _gate = embedder.gate.lock().await;
            let idle = embedder
                .last_used
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_some_and(|t| t.elapsed() >= IDLE_STOP);
            if idle && embedder.server().is_some() {
                embedder.stop_now();
            }
        }
    });
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    #[serde(default)]
    index: usize,
    embedding: Vec<f32>,
}

async fn request(port: u16, key: &str, input: &[String]) -> AppResult<Vec<Vec<f32>>> {
    let fail = |e: String| AppError::Document(format!("Document search failed: {e}"));
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .no_proxy()
        .build()
        .map_err(|e| fail(e.to_string()))?;
    let resp = client
        .post(format!("http://127.0.0.1:{port}/v1/embeddings"))
        .bearer_auth(key)
        .json(&serde_json::json!({ "input": input, "model": EMBED_MODEL_ID }))
        .send()
        .await
        .map_err(|e| fail(e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(fail(format!("{status} {}", body.chars().take(300).collect::<String>())));
    }
    let mut parsed: EmbeddingResponse = resp.json().await.map_err(|e| fail(e.to_string()))?;
    if parsed.data.len() != input.len() {
        return Err(fail(format!("{} answers for {} texts", parsed.data.len(), input.len())));
    }
    parsed.data.sort_by_key(|d| d.index);
    Ok(parsed.data.into_iter().map(|d| normalise(d.embedding)).collect())
}

/// Length 1, so the dot product is the cosine similarity.
pub fn normalise(mut v: Vec<f32>) -> Vec<f32> {
    let len = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if len > 0.0 {
        v.iter_mut().for_each(|x| *x /= len);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_model_is_valid() {
        let m = embed_model();
        m.validate().unwrap();
        assert!(m.tags.iter().any(|t| t == EMBED_TAG));
    }

    #[test]
    fn prefixes() {
        assert_eq!(with_prefix(&Purpose::Document, "abc"), "search_document: abc");
        let long = "x".repeat(5000);
        assert_eq!(with_prefix(&Purpose::Query, &long).len(), "search_query: ".len() + MAX_QUERY_CHARS);
    }

    #[test]
    fn vectors_get_length_one() {
        let v = normalise(vec![3.0, 4.0]);
        assert!((v[0] - 0.6).abs() < 1e-6 && (v[1] - 0.8).abs() < 1e-6);
        assert_eq!(normalise(vec![0.0, 0.0]), vec![0.0, 0.0]);
    }
}
