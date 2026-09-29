mod cpu_ram;
mod gpu;

use serde::Serialize;

use crate::error::{AppError, AppResult};
pub use gpu::{GpuInfo, GpuVendor};

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub os_name: String,
    pub ram_total_bytes: u64,
    pub ram_available_bytes: u64,
    pub cpu_name: String,
    pub cpu_physical_cores: Option<usize>,
    pub cpu_threads: usize,
    pub gpus: Vec<GpuInfo>,
}

pub fn detect() -> DeviceInfo {
    let cpu_ram = cpu_ram::detect();
    DeviceInfo {
        os_name: crate::platform::os_name(),
        ram_total_bytes: cpu_ram.ram_total_bytes,
        ram_available_bytes: cpu_ram.ram_available_bytes,
        cpu_name: cpu_ram.cpu_name,
        cpu_physical_cores: cpu_ram.cpu_physical_cores,
        cpu_threads: cpu_ram.cpu_threads,
        gpus: gpu::detect(),
    }
}

#[cfg(test)]
mod tests {
    /// Prints what this computer reports. Run with:
    /// `cargo test print_this_device -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn print_this_device() {
        println!("{:#?}", super::detect());
    }
}

/// GPU detection runs external programs, so it is kept off the main thread.
pub async fn detect_async() -> AppResult<DeviceInfo> {
    tauri::async_runtime::spawn_blocking(detect)
        .await
        .map_err(|e| AppError::Internal(format!("device check failed: {e}")))
}

#[tauri::command]
pub async fn get_device_info() -> AppResult<DeviceInfo> {
    detect_async().await
}
