//! Starts llama-server and waits until it can answer.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::sync::watch;

use super::logs::{self, LogBuffer};

/// Starts the program hidden (no window, no terminal) and collects its output.
pub fn spawn(exe: &Path, args: &[String], logs: &LogBuffer) -> std::io::Result<Child> {
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let mut cmd = Command::new(exe);
    cmd.args(args)
        // llama-server also looks for its CPU/GPU backends here.
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Linux: die with the app. Windows: no console window.
    crate::platform::prepare_command(&mut cmd);
    let mut child = cmd.spawn()?;
    // Windows: join the app's job object, so it ends when the app ends.
    if let Err(e) = crate::platform::after_spawn(&child) {
        let _ = child.start_kill();
        return Err(e);
    }
    if let Some(out) = child.stdout.take() {
        logs::collect(out, logs.clone());
    }
    if let Some(err) = child.stderr.take() {
        logs::collect(err, logs.clone());
    }
    Ok(child)
}

/// A free port on this computer, chosen by the system.
pub fn free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

/// `true` if nothing else listens on this address and port now.
pub fn port_is_free(host: &str, port: u16) -> bool {
    std::net::TcpListener::bind((host, port)).is_ok()
}

#[derive(Debug, PartialEq, Eq)]
pub enum Wait {
    Ready,
    Exited,
    TimedOut,
}

/// Polls `GET /health` until it answers 200, the process ends, or the time runs out.
/// (`/health` answers 503 while the model is still loading.)
pub async fn wait_until_ready(port: u16, mut exited: watch::Receiver<bool>, timeout: Duration) -> Wait {
    let url = format!("http://127.0.0.1:{port}/health");
    let Ok(client) = reqwest::Client::builder().timeout(Duration::from_secs(2)).no_proxy().build() else {
        return Wait::Exited;
    };
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if *exited.borrow() {
            return Wait::Exited;
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Wait::Ready;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Wait::TimedOut;
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(300)) => {}
            _ = exited.changed() => {}
        }
    }
}
