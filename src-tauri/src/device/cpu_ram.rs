use sysinfo::{CpuRefreshKind, System};

pub struct CpuRam {
    pub ram_total_bytes: u64,
    pub ram_available_bytes: u64,
    pub cpu_name: String,
    pub cpu_physical_cores: Option<usize>,
    pub cpu_threads: usize,
}

pub fn detect() -> CpuRam {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_list(CpuRefreshKind::nothing());

    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown processor".to_string());

    CpuRam {
        ram_total_bytes: sys.total_memory(),
        ram_available_bytes: sys.available_memory(),
        cpu_name,
        cpu_physical_cores: System::physical_core_count(),
        cpu_threads: sys.cpus().len(),
    }
}
