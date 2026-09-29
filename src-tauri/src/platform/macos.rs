//! macOS device detection and process control (Apple Silicon).
//!
//! Apple Silicon has one GPU inside the chip. It shares the main memory
//! (unified memory) and llama.cpp uses it through Metal. Intel Macs are not
//! supported: the app ships only the arm64 engine.

use std::process::Command;

use crate::device::{GpuInfo, GpuVendor};

pub fn os_name() -> String {
    sysinfo::System::long_os_version().unwrap_or_else(|| "macOS".to_string())
}

pub fn detect_gpus() -> Vec<GpuInfo> {
    if !cfg!(target_arch = "aarch64") {
        return Vec::new();
    }
    // "Apple M2 Pro" → "Apple M2 Pro GPU".
    let chip = Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Apple".to_string());
    vec![GpuInfo {
        name: format!("{chip} GPU"),
        vendor: GpuVendor::Apple,
        integrated: true,
        vram_total_bytes: None,
        vram_free_bytes: None,
    }]
}

// ---------- processes ----------

/// macOS has no PR_SET_PDEATHSIG. The app stops llama-server when it closes
/// (`Engine::shutdown_blocking`); only a crash of the app can leave it running.
pub fn prepare_command(_cmd: &mut tokio::process::Command) {}

pub fn after_spawn(_child: &tokio::process::Child) -> std::io::Result<()> {
    Ok(())
}

/// Asks the process to stop (SIGTERM).
pub fn terminate(pid: u32) {
    // SAFETY: sending a signal has no memory effects in this process.
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
}

/// Stops the process at once (SIGKILL).
pub fn kill_now(pid: u32) {
    // SAFETY: as above.
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
}
