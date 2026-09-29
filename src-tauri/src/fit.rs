//! The "Does this model fit my computer?" formula. Change the rules here only.
//!
//! need = model size + EXTRA_MEMORY_BYTES
//!
//! With a separate graphics card that reports its memory (VRAM):
//!   need <= free VRAM         → Fits well
//!   need <= free RAM          → May be slow   (runs on the processor)
//!   otherwise                 → Will not fit
//!
//! Without such a card (processor only, built-in graphics, or VRAM unknown):
//!   need <= free RAM          → Fits well
//!   otherwise                 → Will not fit

use serde::Serialize;

use crate::device::DeviceInfo;

/// Extra memory for the conversation, the engine and the app (about 1.5 GB).
pub const EXTRA_MEMORY_BYTES: u64 = 1536 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FitLabel {
    FitsWell,
    MayBeSlow,
    WillNotFit,
}

pub fn fit_label(model_size_bytes: u64, device: &DeviceInfo) -> FitLabel {
    let need = model_size_bytes.saturating_add(EXTRA_MEMORY_BYTES);
    let free_ram = device.ram_available_bytes;

    match best_gpu_memory(device) {
        Some(vram) if need <= vram => FitLabel::FitsWell,
        Some(_) if need <= free_ram => FitLabel::MayBeSlow,
        Some(_) => FitLabel::WillNotFit,
        None if need <= free_ram => FitLabel::FitsWell,
        None => FitLabel::WillNotFit,
    }
}

/// The largest usable memory of a separate graphics card.
/// Uses free memory when known, else total memory.
fn best_gpu_memory(device: &DeviceInfo) -> Option<u64> {
    device
        .gpus
        .iter()
        .filter(|g| !g.integrated)
        .filter_map(|g| g.vram_free_bytes.or(g.vram_total_bytes))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{GpuInfo, GpuVendor};

    const GB: u64 = 1024 * 1024 * 1024;

    fn device(free_ram_gb: u64, gpu: Option<(bool, Option<u64>)>) -> DeviceInfo {
        DeviceInfo {
            os_name: "test".into(),
            ram_total_bytes: 32 * GB,
            ram_available_bytes: free_ram_gb * GB,
            cpu_name: "cpu".into(),
            cpu_physical_cores: Some(4),
            cpu_threads: 8,
            gpus: gpu
                .map(|(integrated, vram_gb)| GpuInfo {
                    name: "gpu".into(),
                    vendor: GpuVendor::Nvidia,
                    integrated,
                    vram_total_bytes: vram_gb.map(|v| v * GB),
                    vram_free_bytes: vram_gb.map(|v| v * GB),
                })
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn with_graphics_card() {
        let d = device(8, Some((false, Some(6))));
        assert_eq!(fit_label(4 * GB, &d), FitLabel::FitsWell); // 5.5 <= 6
        assert_eq!(fit_label(5 * GB, &d), FitLabel::MayBeSlow); // 6.5 > 6, <= 8
        assert_eq!(fit_label(7 * GB, &d), FitLabel::WillNotFit); // 8.5 > 8
    }

    #[test]
    fn processor_only() {
        let d = device(8, None);
        assert_eq!(fit_label(6 * GB, &d), FitLabel::FitsWell);
        assert_eq!(fit_label(7 * GB, &d), FitLabel::WillNotFit);
    }

    #[test]
    fn built_in_graphics_and_unknown_vram_use_ram() {
        assert_eq!(fit_label(6 * GB, &device(8, Some((true, Some(1))))), FitLabel::FitsWell);
        assert_eq!(fit_label(6 * GB, &device(8, Some((false, None)))), FitLabel::FitsWell);
    }
}
