//! Tests with the real llama-server and the SmolLM2 model (downloaded in Phase 3).
//! Run with: `cargo test engine::tests -- --ignored --nocapture --test-threads 1`

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::watch;

use super::args::{build_args, GpuMode, LaunchConfig};
use super::logs::{explain_failure, LogBuffer};
use super::process::{self, Wait};

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources/llama")
        .join(format!("llama-server{}", std::env::consts::EXE_SUFFIX))
}

fn smollm() -> PathBuf {
    crate::paths::models_dir()
        .join("smollm2-135m-instruct-q4km")
        .join("SmolLM2-135M-Instruct-Q4_K_M.gguf")
}

fn config(model: PathBuf, port: u16) -> LaunchConfig {
    LaunchConfig {
        model_path: model,
        mmproj_path: None,
        host: "127.0.0.1".into(),
        port,
        api_key: Some("test-key".into()),
        model_context: Some(2048),
        gpu: GpuMode::Off,
    }
}

fn is_alive(pid: u32) -> bool {
    // A zombie counts as ended: check the state letter in /proc.
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map(|s| !s.split_whitespace().nth(2).is_some_and(|st| st == "Z"))
        .unwrap_or(false)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn start_chat_and_stop() {
    let port = process::free_port().unwrap();
    let logs = LogBuffer::default();
    let mut child = process::spawn(&exe(), &build_args(&config(smollm(), port)), &logs).unwrap();
    let pid = child.id().unwrap();
    let (tx, rx) = watch::channel(false);

    let started = std::time::Instant::now();
    assert_eq!(process::wait_until_ready(port, rx, Duration::from_secs(60)).await, Wait::Ready);
    println!("ready after {:?}", started.elapsed());

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let body = r#"{"messages":[{"role":"user","content":"Say hi."}],"max_tokens":8}"#;
    let no_key = client.post(&url).body(body).header("Content-Type", "application/json").send().await.unwrap();
    assert_eq!(no_key.status().as_u16(), 401, "API must need the key");

    let ok = client
        .post(&url)
        .bearer_auth("test-key")
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .unwrap();
    assert!(ok.status().is_success());
    println!("answer: {}", ok.text().await.unwrap().chars().take(200).collect::<String>());

    crate::platform::terminate(pid);
    let status = tokio::time::timeout(Duration::from_secs(5), child.wait()).await.unwrap().unwrap();
    let _ = tx.send(true);
    println!("stopped: {status}");
    assert!(!is_alive(pid));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn damaged_model_gives_clear_message() {
    let bad = std::env::temp_dir().join(format!("kuttylm-bad-{}.gguf", std::process::id()));
    std::fs::write(&bad, b"this is not a gguf file").unwrap();
    let port = process::free_port().unwrap();
    let logs = LogBuffer::default();
    let mut child = process::spawn(&exe(), &build_args(&config(bad.clone(), port)), &logs).unwrap();

    let (tx, rx) = watch::channel(false);
    let waiter = tokio::spawn(async move {
        let _ = child.wait().await;
        let _ = tx.send(true);
    });
    assert_eq!(process::wait_until_ready(port, rx, Duration::from_secs(30)).await, Wait::Exited);
    waiter.await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await; // let the log readers finish
    let message = explain_failure(&logs.text());
    println!("message: {message}\nlog tail: {:?}", logs.tail(3));
    assert!(message.contains("damaged"), "{message}");
    let _ = std::fs::remove_file(bad);
}

/// If the thread that started llama-server ends (as when the app crashes),
/// the kernel must kill llama-server.
#[test]
#[ignore]
fn child_dies_with_its_parent_thread() {
    let port = process::free_port().unwrap();
    let args = build_args(&config(smollm(), port));
    let (pid_tx, pid_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let child = process::spawn(&exe(), &args, &LogBuffer::default()).unwrap();
            pid_tx.send(child.id().unwrap()).unwrap();
            std::mem::forget(child); // no kill_on_drop: only the kernel may stop it
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        // The thread ends here, like a crashed app.
    })
    .join()
    .unwrap();

    let pid = pid_rx.recv().unwrap();
    for _ in 0..50 {
        if !is_alive(pid) {
            println!("llama-server {pid} ended with its parent thread");
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    crate::platform::kill_now(pid);
    panic!("llama-server {pid} was still running");
}
