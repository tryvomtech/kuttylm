//! Linux device detection.
//!
//! GPU order of checks:
//! 1. `nvidia-smi` for NVIDIA cards (name + VRAM).
//! 2. `/sys/class/drm/cardN/device` for all other cards
//!    (AMD cards report VRAM in `mem_info_vram_*`).
//! 3. `lspci` for a readable card name.
//! 4. `vulkaninfo --summary` for names when nothing else worked.

use std::fs;
use std::path::Path;

use super::common::{nvidia_smi_gpus, run};
use crate::device::{GpuInfo, GpuVendor};

pub fn os_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| parse_os_release_name(&text))
        .unwrap_or_else(|| "Linux".to_string())
}

pub fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = nvidia_smi_gpus();
    let have_nvidia_smi = !gpus.is_empty();

    for card in sysfs_cards() {
        if have_nvidia_smi && card.vendor == GpuVendor::Nvidia {
            continue;
        }
        gpus.push(card);
    }

    if gpus.is_empty() {
        gpus = vulkan_gpus();
    }
    gpus
}

// ---------- sysfs + lspci ----------

fn sysfs_cards() -> Vec<GpuInfo> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| is_card_dir(n))
        .collect();
    names.sort();

    let mut seen_slots = Vec::new();
    let mut cards = Vec::new();
    for name in names {
        let device_dir = Path::new("/sys/class/drm").join(&name).join("device");
        let Some(slot) = fs::canonicalize(&device_dir)
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()))
        else {
            continue;
        };
        if seen_slots.contains(&slot) {
            continue;
        }
        seen_slots.push(slot.clone());

        let vendor = read_hex(&device_dir.join("vendor"))
            .map(GpuVendor::from_pci_id)
            .unwrap_or(GpuVendor::Other);
        let vram_total = read_u64(&device_dir.join("mem_info_vram_total"));
        let vram_used = read_u64(&device_dir.join("mem_info_vram_used"));
        let integrated = is_integrated_slot(&slot);

        let name = run("lspci", &["-mm", "-s", &slot])
            .and_then(|out| parse_lspci_name(&out, vendor))
            .unwrap_or_else(|| format!("{} graphics", vendor_label(vendor)));

        cards.push(GpuInfo {
            name,
            vendor,
            integrated,
            // Integrated GPUs share system memory; their small "VRAM" number is misleading.
            vram_total_bytes: if integrated { None } else { vram_total },
            vram_free_bytes: if integrated {
                None
            } else {
                vram_total.zip(vram_used).map(|(t, u)| t.saturating_sub(u))
            },
        });
    }
    cards
}

/// `card0`, `card1`… but not `card1-HDMI-A-1`.
fn is_card_dir(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

/// GPUs built into the processor sit on PCI bus 0 (`0000:00:02.0`).
/// Plug-in cards sit on a higher bus number.
fn is_integrated_slot(slot: &str) -> bool {
    let parts: Vec<&str> = slot.split(':').collect();
    let bus = if parts.len() == 3 { parts[1] } else { parts.first().copied().unwrap_or("") };
    u8::from_str_radix(bus, 16).is_ok_and(|b| b == 0)
}

/// Parses one `lspci -mm` line, for example:
/// `00:02.0 "VGA compatible controller" "Intel Corporation" "CometLake-U GT2 [UHD Graphics]" -r02 "Dell" "Latitude 7410"`
/// and returns a short name like `Intel UHD Graphics`.
fn parse_lspci_name(output: &str, vendor: GpuVendor) -> Option<String> {
    let line = output.lines().next()?;
    let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
    let device = quoted.get(2)?.trim();
    if device.is_empty() {
        return None;
    }
    let model = match (device.rfind('['), device.rfind(']')) {
        (Some(start), Some(end)) if end > start + 1 => &device[start + 1..end],
        _ => device,
    };
    Some(format!("{} {}", vendor_label(vendor), model))
}

fn vendor_label(vendor: GpuVendor) -> &'static str {
    match vendor {
        GpuVendor::Nvidia => "NVIDIA",
        GpuVendor::Amd => "AMD",
        GpuVendor::Intel => "Intel",
        GpuVendor::Apple => "Apple",
        GpuVendor::Other => "Unknown",
    }
}

// ---------- Vulkan (last fallback) ----------

fn vulkan_gpus() -> Vec<GpuInfo> {
    let Some(out) = run("vulkaninfo", &["--summary"]) else {
        return Vec::new();
    };
    parse_vulkan_summary(&out)
}

fn parse_vulkan_summary(output: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut device_type = "";
    for line in output.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "deviceType" => device_type = value,
            "deviceName" => {
                // Skip software renderers such as llvmpipe.
                if device_type.ends_with("_CPU") {
                    continue;
                }
                let lower = value.to_lowercase();
                let vendor = if lower.contains("nvidia") || lower.contains("geforce") {
                    GpuVendor::Nvidia
                } else if lower.contains("amd") || lower.contains("radeon") {
                    GpuVendor::Amd
                } else if lower.contains("intel") {
                    GpuVendor::Intel
                } else {
                    GpuVendor::Other
                };
                gpus.push(GpuInfo {
                    name: value.to_string(),
                    vendor,
                    integrated: device_type.ends_with("_INTEGRATED_GPU"),
                    vram_total_bytes: None,
                    vram_free_bytes: None,
                });
            }
            _ => {}
        }
    }
    gpus
}

// ---------- processes ----------

/// Before start: the kernel sends SIGKILL to llama-server when the thread that
/// started it ends, so it cannot outlive a crashed app. Spawn only from a
/// long-lived thread (a tokio worker, not `spawn_blocking`), because the
/// signal follows the thread, not the process.
pub fn prepare_command(cmd: &mut tokio::process::Command) {
    // SAFETY: only calls prctl, which is safe between fork and exec.
    unsafe {
        cmd.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong, 0, 0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

/// Nothing to do after start on Linux (see `prepare_command`).
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

// ---------- helpers ----------

fn parse_os_release_name(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_hex(path: &Path) -> Option<u16> {
    let text = fs::read_to_string(path).ok()?;
    u16::from_str_radix(text.trim().trim_start_matches("0x"), 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lspci_names() {
        let intel = r#"00:02.0 "VGA compatible controller" "Intel Corporation" "CometLake-U GT2 [UHD Graphics]" -r02 "Dell" "Latitude 7410""#;
        assert_eq!(parse_lspci_name(intel, GpuVendor::Intel).unwrap(), "Intel UHD Graphics");

        let amd = r#"03:00.0 "VGA compatible controller" "Advanced Micro Devices, Inc. [AMD/ATI]" "Navi 23 [Radeon RX 6600/6600 XT/6600M]" -rc7 "" """#;
        assert_eq!(
            parse_lspci_name(amd, GpuVendor::Amd).unwrap(),
            "AMD Radeon RX 6600/6600 XT/6600M"
        );

        let plain = r#"01:00.0 "3D controller" "Some Vendor" "Mystery GPU" "" """#;
        assert_eq!(parse_lspci_name(plain, GpuVendor::Other).unwrap(), "Unknown Mystery GPU");
    }

    #[test]
    fn card_dirs() {
        assert!(is_card_dir("card0"));
        assert!(is_card_dir("card12"));
        assert!(!is_card_dir("card1-HDMI-A-1"));
        assert!(!is_card_dir("renderD128"));
        assert!(!is_card_dir("card"));
    }

    #[test]
    fn integrated_slots() {
        assert!(is_integrated_slot("0000:00:02.0"));
        assert!(!is_integrated_slot("0000:01:00.0"));
        assert!(!is_integrated_slot("0000:0a:00.0"));
    }

    #[test]
    fn vulkan_summary_skips_cpu_renderer() {
        let text = "\
            deviceType         = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU\n\
            deviceName         = Intel(R) UHD Graphics (CML GT2)\n\
            deviceType         = PHYSICAL_DEVICE_TYPE_CPU\n\
            deviceName         = llvmpipe (LLVM 15.0.7, 256 bits)\n";
        let gpus = parse_vulkan_summary(text);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, GpuVendor::Intel);
        assert!(gpus[0].integrated);
    }

    #[test]
    fn os_release() {
        let text = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 22.04.5 LTS\"\n";
        assert_eq!(parse_os_release_name(text).unwrap(), "Ubuntu 22.04.5 LTS");
    }
}
