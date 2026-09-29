//! Keeps the last lines of llama-server output, to show them when it fails.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

const MAX_LINES: usize = 300;

#[derive(Clone, Default)]
pub struct LogBuffer(Arc<Mutex<VecDeque<String>>>);

impl LogBuffer {
    fn lock(&self) -> std::sync::MutexGuard<'_, VecDeque<String>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn push(&self, line: String) {
        let mut lines = self.lock();
        if lines.len() == MAX_LINES {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    pub fn clear(&self) {
        self.lock().clear();
    }

    /// The last `n` lines, oldest first.
    pub fn tail(&self, n: usize) -> Vec<String> {
        let lines = self.lock();
        lines.iter().skip(lines.len().saturating_sub(n)).cloned().collect()
    }

    pub fn text(&self) -> String {
        self.lock().iter().map(String::as_str).collect::<Vec<_>>().join("\n")
    }
}

/// Copies every line of a child's stdout or stderr into the buffer.
pub fn collect<R: AsyncRead + Unpin + Send + 'static>(reader: R, logs: LogBuffer) {
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            logs.push(line);
        }
    });
}

/// Turns a failed start into a message a beginner can act on.
pub fn explain_failure(log: &str) -> &'static str {
    let lower = log.to_lowercase();
    if lower.contains("out of memory")
        || lower.contains("failed to allocate")
        || lower.contains("unable to allocate")
        || lower.contains("cannot allocate memory")
    {
        "There is not enough memory to load this model. Close other programs or choose a smaller model."
    } else if lower.contains("failed to load model") || lower.contains("error loading model") {
        "The model file could not be loaded. It may be damaged. Try deleting it and downloading it again."
    } else if lower.contains("address already in use") || lower.contains("couldn't bind") {
        "The port is used by another program. Choose another port in Settings."
    } else {
        "The model could not start. See the details below."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_last_lines() {
        let logs = LogBuffer::default();
        for i in 0..(MAX_LINES + 5) {
            logs.push(format!("line {i}"));
        }
        assert_eq!(logs.tail(2), vec![format!("line {}", MAX_LINES + 3), format!("line {}", MAX_LINES + 4)]);
        assert_eq!(logs.tail(10_000).len(), MAX_LINES);
    }

    #[test]
    fn explains_failures() {
        assert!(explain_failure("ggml_vulkan: Device memory allocation failed: out of memory").contains("memory"));
        assert!(explain_failure("llama_model_load: error loading model: bad magic").contains("damaged"));
        assert!(explain_failure("couldn't bind HTTP server socket").contains("port"));
        assert!(explain_failure("??").contains("could not start"));
    }
}
