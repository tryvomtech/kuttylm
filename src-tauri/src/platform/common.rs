//! Helpers shared by Linux and Windows.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::device::{GpuInfo, GpuVendor};

/// PowerShell can take a few seconds to start.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const MIB: u64 = 1024 * 1024;

// ---------- NVIDIA ----------

pub fn nvidia_smi_gpus() -> Vec<GpuInfo> {
    let Some(out) = run(
        "nvidia-smi",
        &[
            "--query-gpu=name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ],
    ) else {
        return Vec::new();
    };
    out.lines().filter_map(parse_nvidia_smi_line).collect()
}

fn parse_nvidia_smi_line(line: &str) -> Option<GpuInfo> {
    let mut parts = line.split(',').map(str::trim);
    let name = parts.next().filter(|s| !s.is_empty())?.to_string();
    let total_mib = parts.next().and_then(|s| s.parse::<u64>().ok());
    let free_mib = parts.next().and_then(|s| s.parse::<u64>().ok());
    Some(GpuInfo {
        name,
        vendor: GpuVendor::Nvidia,
        integrated: false,
        vram_total_bytes: total_mib.map(|m| m * MIB),
        vram_free_bytes: free_mib.map(|m| m * MIB),
    })
}

/// Runs a program and returns its stdout. Returns `None` if the program is
/// missing, fails, or takes longer than `COMMAND_TIMEOUT` (a broken GPU driver
/// can make `nvidia-smi` hang).
pub fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no black console window flashes up.
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // Read stdout on another thread so a full pipe cannot block the child.
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        stdout.read_to_string(&mut out).ok().map(|_| out)
    });

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = reader.join().ok().flatten();
                return if status.success() { out } else { None };
            }
            Ok(None) if start.elapsed() < COMMAND_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_line() {
        let gpu = parse_nvidia_smi_line("NVIDIA GeForce RTX 3060, 12288, 11800").unwrap();
        assert_eq!(gpu.name, "NVIDIA GeForce RTX 3060");
        assert_eq!(gpu.vram_total_bytes, Some(12288 * MIB));
        assert_eq!(gpu.vram_free_bytes, Some(11800 * MIB));
    }

    #[test]
    fn nvidia_line_without_numbers() {
        let gpu = parse_nvidia_smi_line("NVIDIA T400, [N/A], [N/A]").unwrap();
        assert_eq!(gpu.vram_total_bytes, None);
    }
}
