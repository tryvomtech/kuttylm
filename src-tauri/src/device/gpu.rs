use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
// macOS only reports Apple GPUs.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    /// Apple Silicon (M1, M2…): the GPU inside the chip, used through Metal.
    Apple,
    Other,
}

impl GpuVendor {
    #[cfg(target_os = "linux")]
    pub fn from_pci_id(id: u16) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 => Self::Amd,
            0x8086 => Self::Intel,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: GpuVendor,
    /// Built into the processor. It uses system RAM, not its own VRAM.
    pub integrated: bool,
    /// `None` when the size of the GPU memory cannot be found.
    pub vram_total_bytes: Option<u64>,
    pub vram_free_bytes: Option<u64>,
}

pub fn detect() -> Vec<GpuInfo> {
    crate::platform::detect_gpus()
}
