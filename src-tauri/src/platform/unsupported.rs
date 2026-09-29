use crate::device::GpuInfo;

pub fn os_name() -> String {
    std::env::consts::OS.to_string()
}

pub fn detect_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

pub fn prepare_command(_cmd: &mut tokio::process::Command) {}

pub fn after_spawn(_child: &tokio::process::Child) -> std::io::Result<()> {
    Ok(())
}

pub fn terminate(_pid: u32) {}

pub fn kill_now(_pid: u32) {}
