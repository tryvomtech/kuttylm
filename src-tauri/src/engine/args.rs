//! Command-line options for llama-server.

use std::path::PathBuf;

use crate::device::GpuVendor;
use crate::settings::GpuSetting;

/// Largest context the app asks for. Some models allow 131072 tokens,
/// which would need many GB of memory for the conversation alone.
pub const MAX_CONTEXT: u64 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMode {
    /// llama-server picks the layers and fits them into GPU memory.
    Auto,
    /// Processor only.
    Off,
    /// A fixed number of layers on the graphics card.
    Layers(u32),
}

/// "Auto" in the app: use the graphics card only when there is a separate one.
/// Built-in GPUs share the main memory and were much slower than the
/// processor in tests (Intel UHD: 5 vs 96 tokens/s reading the prompt).
/// Apple Silicon is the exception: its GPU (Metal) is faster than its processor.
pub fn auto_gpu_mode(device: &crate::device::DeviceInfo) -> GpuMode {
    if device.gpus.iter().any(|g| !g.integrated || g.vendor == GpuVendor::Apple) {
        GpuMode::Auto
    } else {
        GpuMode::Off
    }
}

/// Turns the Settings choice into the first mode to try.
pub fn gpu_mode_for(setting: GpuSetting, layers: u32, device: &crate::device::DeviceInfo) -> GpuMode {
    match setting {
        GpuSetting::Auto => auto_gpu_mode(device),
        GpuSetting::Gpu => GpuMode::Auto,
        GpuSetting::Cpu => GpuMode::Off,
        GpuSetting::Custom => GpuMode::Layers(layers),
    }
}

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// The first file of the model (llama-server finds split parts by itself).
    pub model_path: PathBuf,
    pub mmproj_path: Option<PathBuf>,
    pub host: String,
    pub port: u16,
    /// `None` = no key needed (API on without a key).
    pub api_key: Option<String>,
    /// From the model list; `None` lets llama-server read it from the file.
    pub model_context: Option<u64>,
    pub gpu: GpuMode,
}

pub fn build_args(cfg: &LaunchConfig) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--model".into(),
        cfg.model_path.to_string_lossy().into_owned(),
        "--host".into(),
        cfg.host.clone(),
        "--port".into(),
        cfg.port.to_string(),
        "--ctx-size".into(),
        cfg.model_context.map_or(MAX_CONTEXT, |c| c.min(MAX_CONTEXT)).to_string(),
        // Our own UI talks to the API; the built-in web page is not needed.
        "--no-webui".into(),
        // Never download anything by itself.
        "--offline".into(),
        // Plain text logs, so the UI can show them.
        "--log-colors".into(),
        "off".into(),
    ];
    if let Some(key) = &cfg.api_key {
        args.push("--api-key".into());
        args.push(key.clone());
    }
    if let Some(mmproj) = &cfg.mmproj_path {
        args.push("--mmproj".into());
        args.push(mmproj.to_string_lossy().into_owned());
    }
    match cfg.gpu {
        GpuMode::Auto => {}
        GpuMode::Off => args.extend(["--device".into(), "none".into(), "--gpu-layers".into(), "0".into()]),
        GpuMode::Layers(n) => args.extend(["--gpu-layers".into(), n.to_string()]),
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> LaunchConfig {
        LaunchConfig {
            model_path: "/m/a.gguf".into(),
            mmproj_path: None,
            host: "127.0.0.1".into(),
            port: 4000,
            api_key: Some("k".into()),
            model_context: Some(131072),
            gpu: GpuMode::Auto,
        }
    }

    fn value_of(args: &[String], flag: &str) -> Option<String> {
        args.iter().position(|a| a == flag).map(|i| args[i + 1].clone())
    }

    #[test]
    fn basic_args() {
        let args = build_args(&cfg());
        assert_eq!(value_of(&args, "--model").unwrap(), "/m/a.gguf");
        assert_eq!(value_of(&args, "--port").unwrap(), "4000");
        assert_eq!(value_of(&args, "--api-key").unwrap(), "k");
        assert_eq!(value_of(&args, "--ctx-size").unwrap(), "8192");
        assert!(!args.contains(&"--device".to_string()));
        assert!(!args.contains(&"--mmproj".to_string()));
    }

    #[test]
    fn no_key_and_fixed_layers() {
        let mut c = cfg();
        c.api_key = None;
        c.gpu = GpuMode::Layers(12);
        let args = build_args(&c);
        assert!(!args.contains(&"--api-key".to_string()));
        assert_eq!(value_of(&args, "--gpu-layers").unwrap(), "12");
        assert!(!args.contains(&"--device".to_string()));
    }

    #[test]
    fn small_context_is_kept() {
        let mut c = cfg();
        c.model_context = Some(4096);
        assert_eq!(value_of(&build_args(&c), "--ctx-size").unwrap(), "4096");
    }

    #[test]
    fn cpu_only_and_vision() {
        let mut c = cfg();
        c.gpu = GpuMode::Off;
        c.mmproj_path = Some("/m/mmproj.gguf".into());
        let args = build_args(&c);
        assert_eq!(value_of(&args, "--device").unwrap(), "none");
        assert_eq!(value_of(&args, "--gpu-layers").unwrap(), "0");
        assert_eq!(value_of(&args, "--mmproj").unwrap(), "/m/mmproj.gguf");
    }

    fn device_with(vendor: GpuVendor, integrated: bool) -> crate::device::DeviceInfo {
        crate::device::DeviceInfo {
            os_name: "test".into(),
            ram_total_bytes: 0,
            ram_available_bytes: 0,
            cpu_name: "cpu".into(),
            cpu_physical_cores: None,
            cpu_threads: 1,
            gpus: vec![crate::device::GpuInfo {
                name: "gpu".into(),
                vendor,
                integrated,
                vram_total_bytes: None,
                vram_free_bytes: None,
            }],
        }
    }

    #[test]
    fn auto_uses_separate_cards_and_apple_silicon() {
        assert_eq!(auto_gpu_mode(&device_with(GpuVendor::Nvidia, false)), GpuMode::Auto);
        assert_eq!(auto_gpu_mode(&device_with(GpuVendor::Intel, true)), GpuMode::Off);
        assert_eq!(auto_gpu_mode(&device_with(GpuVendor::Apple, true)), GpuMode::Auto);
    }
}
