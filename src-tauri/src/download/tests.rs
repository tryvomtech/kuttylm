//! Download task tests against a small local HTTP server (no internet needed).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;

use super::task::{self, Outcome};
use super::{verify, Control};
use crate::catalog::types::{CatalogFile, CatalogModel};
use crate::error::AppError;

#[derive(Clone, Default)]
struct ServerOptions {
    /// Answer with this status and no body.
    status: Option<u16>,
    /// Send the whole file (200) even when a Range is asked for.
    ignore_range: bool,
    /// Sleep between 16 KB chunks.
    chunk_delay: Option<Duration>,
    /// Close the connection after this many body bytes.
    cut_after: Option<usize>,
}

struct TestServer {
    base_url: String,
    /// The Range start of each request (`None` = no Range header).
    ranges: Arc<Mutex<Vec<Option<u64>>>>,
}

async fn start_server(body: Vec<u8>, opts: ServerOptions) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let seen = ranges.clone();
    let body = Arc::new(body);

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { return };
            let (body, opts, seen) = (body.clone(), opts.clone(), seen.clone());
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    let Ok(n) = sock.read(&mut tmp).await else { return };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                let head = String::from_utf8_lossy(&buf).to_lowercase();
                let range: Option<u64> = head
                    .lines()
                    .find_map(|l| l.strip_prefix("range: bytes="))
                    .and_then(|r| r.split('-').next()?.trim().parse().ok());
                seen.lock().unwrap().push(range);

                if let Some(status) = opts.status {
                    let resp = format!("HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    let _ = sock.write_all(resp.as_bytes()).await;
                    return;
                }

                let total = body.len();
                let (start, head) = match range {
                    Some(start) if !opts.ignore_range => (
                        start as usize,
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                            total - start as usize, start, total - 1, total
                        ),
                    ),
                    _ => (0, format!("HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n")),
                };
                if sock.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                let mut sent = 0;
                for chunk in body[start..].chunks(16 * 1024) {
                    if let Some(cut) = opts.cut_after {
                        if sent >= cut {
                            return; // Drop the connection in the middle.
                        }
                    }
                    if sock.write_all(chunk).await.is_err() {
                        return;
                    }
                    sent += chunk.len();
                    if let Some(d) = opts.chunk_delay {
                        tokio::time::sleep(d).await;
                    }
                }
            });
        }
    });

    TestServer { base_url, ranges }
}

fn test_body(len: usize, seed: u8) -> Vec<u8> {
    (0..len).map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed)).collect()
}

fn sha_of(data: &[u8]) -> String {
    let path = temp_dir("sha").join("f");
    std::fs::write(&path, data).unwrap();
    verify::sha256_hex(&path).unwrap()
}

fn temp_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "kuttylm-dl-{}-{}-{}",
        std::process::id(),
        name,
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn model(files: Vec<(&str, &[u8])>) -> CatalogModel {
    CatalogModel {
        id: "test-model".into(),
        name: "Test".into(),
        description: None,
        hf_repo: "owner/repo".into(),
        revision: "main".into(),
        files: files
            .into_iter()
            .map(|(name, data)| CatalogFile {
                name: name.into(),
                size_bytes: data.len() as u64,
                sha256: Some(sha_of(data)),
            })
            .collect(),
        mmproj: None,
        quantization: None,
        params_b: None,
        context_length: None,
        license: None,
        tags: vec![],
        trending: false,
    }
}

/// Waits until the download has written some bytes (the writer buffers 1 MB,
/// so this checks that the file exists and the transfer has started).
async fn wait_for_part_bytes(part: &std::path::Path) {
    for _ in 0..500 {
        if part.exists() {
            tokio::time::sleep(Duration::from_millis(60)).await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("download did not start");
}

async fn run(m: &CatalogModel, dir: &std::path::Path, server: &TestServer) -> Result<Outcome, AppError> {
    let (_tx, rx) = watch::channel(Control::Run);
    task::run(m, dir, &server.base_url, rx, |_| {}).await
}

#[tokio::test(flavor = "multi_thread")]
async fn fresh_download() {
    let data = test_body(200_000, 1);
    let server = start_server(data.clone(), ServerOptions::default()).await;
    let dir = temp_dir("fresh");
    let m = model(vec![("a.gguf", &data)]);

    assert_eq!(run(&m, &dir, &server).await.unwrap(), Outcome::Done);
    assert_eq!(std::fs::read(dir.join("a.gguf")).unwrap(), data);
    assert!(!dir.join("a.gguf.part").exists());
    assert_eq!(*server.ranges.lock().unwrap(), vec![None]);
}

#[tokio::test(flavor = "multi_thread")]
async fn resume_uses_range() {
    let data = test_body(200_000, 2);
    let server = start_server(data.clone(), ServerOptions::default()).await;
    let dir = temp_dir("resume");
    std::fs::write(dir.join("a.gguf.part"), &data[..50_000]).unwrap();
    let m = model(vec![("a.gguf", &data)]);

    assert_eq!(run(&m, &dir, &server).await.unwrap(), Outcome::Done);
    assert_eq!(std::fs::read(dir.join("a.gguf")).unwrap(), data);
    assert_eq!(*server.ranges.lock().unwrap(), vec![Some(50_000)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn server_ignoring_range_restarts_cleanly() {
    let data = test_body(100_000, 3);
    let opts = ServerOptions { ignore_range: true, ..Default::default() };
    let server = start_server(data.clone(), opts).await;
    let dir = temp_dir("norange");
    std::fs::write(dir.join("a.gguf.part"), &data[..30_000]).unwrap();
    let m = model(vec![("a.gguf", &data)]);

    assert_eq!(run(&m, &dir, &server).await.unwrap(), Outcome::Done);
    assert_eq!(std::fs::read(dir.join("a.gguf")).unwrap(), data);
}

#[tokio::test(flavor = "multi_thread")]
async fn split_files_and_skip_finished_ones() {
    let a = test_body(60_000, 4);
    let b = test_body(40_000, 5);
    // The server always sends `b`; `a` is already finished, so it must not be requested.
    let server = start_server(b.clone(), ServerOptions::default()).await;
    let dir = temp_dir("split");
    std::fs::write(dir.join("m-00001-of-00002.gguf"), &a).unwrap();
    let m = model(vec![("m-00001-of-00002.gguf", &a), ("m-00002-of-00002.gguf", &b)]);

    assert_eq!(run(&m, &dir, &server).await.unwrap(), Outcome::Done);
    assert_eq!(std::fs::read(dir.join("m-00002-of-00002.gguf")).unwrap(), b);
    assert_eq!(server.ranges.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn login_needed() {
    for status in [401, 403] {
        let data = test_body(1_000, 6);
        let server = start_server(data.clone(), ServerOptions { status: Some(status), ..Default::default() }).await;
        let m = model(vec![("a.gguf", &data)]);
        assert!(matches!(run(&m, &temp_dir("login"), &server).await, Err(AppError::NeedsLogin)));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn not_found() {
    let data = test_body(1_000, 7);
    let server = start_server(data.clone(), ServerOptions { status: Some(404), ..Default::default() }).await;
    let m = model(vec![("a.gguf", &data)]);
    assert!(matches!(run(&m, &temp_dir("404"), &server).await, Err(AppError::FileNotFoundOnline)));
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_sha256_removes_the_file() {
    let data = test_body(50_000, 8);
    let server = start_server(data.clone(), ServerOptions::default()).await;
    let dir = temp_dir("badsha");
    let mut m = model(vec![("a.gguf", &data)]);
    m.files[0].sha256 = Some("0".repeat(64));

    assert!(matches!(run(&m, &dir, &server).await, Err(AppError::DamagedFile)));
    assert!(!dir.join("a.gguf.part").exists());
    assert!(!dir.join("a.gguf").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn network_drop_keeps_part_file() {
    let data = test_body(300_000, 9);
    let opts = ServerOptions { cut_after: Some(100_000), ..Default::default() };
    let server = start_server(data.clone(), opts).await;
    let dir = temp_dir("drop");
    let m = model(vec![("a.gguf", &data)]);

    assert!(matches!(run(&m, &dir, &server).await, Err(AppError::Network(_))));
    let part = std::fs::metadata(dir.join("a.gguf.part")).unwrap().len();
    assert!(part > 0 && part < data.len() as u64, "part size {part}");

    // Resume with a good server finishes the same file.
    let good = start_server(data.clone(), ServerOptions::default()).await;
    assert_eq!(run(&m, &dir, &good).await.unwrap(), Outcome::Done);
    assert_eq!(std::fs::read(dir.join("a.gguf")).unwrap(), data);
    assert_eq!(*good.ranges.lock().unwrap(), vec![Some(part)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_then_resume() {
    let data = test_body(400_000, 10);
    let slow = ServerOptions { chunk_delay: Some(Duration::from_millis(20)), ..Default::default() };
    let server = start_server(data.clone(), slow).await;
    let dir = temp_dir("pause");
    let m = model(vec![("a.gguf", &data)]);

    let (tx, rx) = watch::channel(Control::Run);
    let (m2, dir2, url) = (m.clone(), dir.clone(), server.base_url.clone());
    let job = tokio::spawn(async move { task::run(&m2, &dir2, &url, rx, |_| {}).await });
    wait_for_part_bytes(&dir.join("a.gguf.part")).await;
    tx.send_replace(Control::Pause);

    assert_eq!(job.await.unwrap().unwrap(), Outcome::Paused);
    let part = std::fs::metadata(dir.join("a.gguf.part")).unwrap().len();
    assert!(part > 0 && part < data.len() as u64, "part size {part}");

    let fast = start_server(data.clone(), ServerOptions::default()).await;
    assert_eq!(run(&m, &dir, &fast).await.unwrap(), Outcome::Done);
    assert_eq!(std::fs::read(dir.join("a.gguf")).unwrap(), data);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_stops_quickly() {
    let data = test_body(400_000, 11);
    let slow = ServerOptions { chunk_delay: Some(Duration::from_millis(50)), ..Default::default() };
    let server = start_server(data.clone(), slow).await;
    let dir = temp_dir("cancel");
    let m = model(vec![("a.gguf", &data)]);

    let (tx, rx) = watch::channel(Control::Run);
    let url = server.base_url.clone();
    let dir2 = dir.clone();
    let job = tokio::spawn(async move { task::run(&m, &dir2, &url, rx, |_| {}).await });
    wait_for_part_bytes(&dir.join("a.gguf.part")).await;
    let asked = std::time::Instant::now();
    tx.send_replace(Control::Cancel);

    assert_eq!(job.await.unwrap().unwrap(), Outcome::Cancelled);
    assert!(asked.elapsed() < Duration::from_secs(1));
}

/// One real download from Hugging Face into the real models folder (about 105 MB).
/// Pauses at about 30 MB, then resumes. Run with:
/// `cargo test real_download_smallest_model -- --ignored --nocapture`
#[test]
#[ignore]
fn real_download_smallest_model() {
    const ID: &str = "smollm2-135m-instruct-q4km";
    tauri::async_runtime::block_on(async {
        let body = include_str!("../../resources/models.mock.json");
        let model = crate::catalog::parse_for_tests(body)
            .into_iter()
            .find(|m| m.id == ID)
            .expect("model in mock list");
        let dir = crate::library::model_dir(ID).unwrap();
        super::prepare(&model, &dir).unwrap();

        // Part 1: pause after ~30 MB.
        let (tx, rx) = watch::channel(Control::Run);
        let mut paused_once = false;
        let outcome = task::run(&model, &dir, crate::config::HF_BASE_URL, rx, |p| {
            if !paused_once && p.downloaded_bytes > 30_000_000 {
                paused_once = true;
                println!("pausing at {} bytes, speed {} B/s", p.downloaded_bytes, p.speed_bps);
                tx.send_replace(Control::Pause);
            }
        })
        .await
        .unwrap();
        let part = std::fs::metadata(dir.join(format!("{}.part", model.files[0].name))).unwrap().len();
        println!("outcome {outcome:?}, part file {part} bytes");
        assert_eq!(outcome, Outcome::Paused);

        // Part 2: resume to the end, then mark as ready.
        let (_tx, rx) = watch::channel(Control::Run);
        let mut last = 0;
        let outcome = task::run(&model, &dir, crate::config::HF_BASE_URL, rx, |p| {
            if p.downloaded_bytes / 20_000_000 != last / 20_000_000 {
                println!("  {} of {} bytes, {} B/s, eta {:?}", p.downloaded_bytes, p.total_bytes, p.speed_bps, p.eta_secs);
            }
            last = p.downloaded_bytes;
        })
        .await
        .unwrap();
        assert_eq!(outcome, Outcome::Done);
        super::finish(&model, &dir, outcome).unwrap();
        println!("ready: {}", crate::library::is_ready(&dir));
    });
}
