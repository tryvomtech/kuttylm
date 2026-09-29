//! Reads the PowerShell output of the Windows display driver entries.
//! Plain text work, so it is also built (and tested) on other systems.

use serde::Deserialize;

use crate::device::{GpuInfo, GpuVendor};

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct RegGpu {
    name: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    vram: u64,
}

/// PowerShell prints one object (not a list) when there is only one adapter.
pub fn parse_registry_gpus(json: &str) -> Vec<GpuInfo> {
    let json = json.trim();
    if json.is_empty() {
        return Vec::new();
    }
    let list: Vec<RegGpu> = if json.starts_with('[') {
        serde_json::from_str(json).unwrap_or_default()
    } else {
        serde_json::from_str::<RegGpu>(json).map(|g| vec![g]).unwrap_or_default()
    };
    list.into_iter().filter_map(to_gpu_info).collect()
}

fn to_gpu_info(g: RegGpu) -> Option<GpuInfo> {
    let lower = g.name.to_lowercase();
    // Software adapters are not graphics cards.
    if lower.contains("microsoft basic") || lower.contains("remote display") || lower.contains("virtual") {
        return None;
    }
    let id = g.id.to_lowercase();
    let vendor = if id.contains("ven_10de") || lower.contains("nvidia") {
        GpuVendor::Nvidia
    } else if id.contains("ven_1002") || lower.contains("amd") || lower.contains("radeon") {
        GpuVendor::Amd
    } else if id.contains("ven_8086") || lower.contains("intel") {
        GpuVendor::Intel
    } else {
        GpuVendor::Other
    };
    let vram = (g.vram > 0).then_some(g.vram);
    // Built-in graphics: Intel (except Arc cards), or a small memory carve-out.
    let integrated = (vendor == GpuVendor::Intel && !lower.contains("arc")) || vram.is_some_and(|v| v < GIB);
    Some(GpuInfo {
        name: g.name,
        vendor,
        integrated,
        vram_total_bytes: if integrated { None } else { vram },
        vram_free_bytes: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_registry_output() {
        let many = r#"[{"name":"NVIDIA GeForce RTX 3060","id":"pci\\ven_10de&dev_2504","vram":12884901888},
                      {"name":"Intel(R) UHD Graphics 630","id":"pci\\ven_8086&dev_3e92","vram":1073741824},
                      {"name":"Microsoft Basic Display Adapter","id":"","vram":0}]"#;
        let gpus = parse_registry_gpus(many);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].vendor, GpuVendor::Nvidia);
        assert_eq!(gpus[0].vram_total_bytes, Some(12 * GIB));
        assert!(!gpus[0].integrated);
        assert!(gpus[1].integrated);
        assert_eq!(gpus[1].vram_total_bytes, None);

        let one = r#"{"name":"AMD Radeon RX 7600","id":"pci\\ven_1002&dev_7480","vram":8589934592}"#;
        let gpus = parse_registry_gpus(one);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, GpuVendor::Amd);
        assert!(!gpus[0].integrated);
    }
}
