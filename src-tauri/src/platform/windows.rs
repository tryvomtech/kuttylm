//! Windows device detection and process control.
//!
//! GPU order of checks:
//! 1. `nvidia-smi` for NVIDIA cards (name + VRAM).
//! 2. The display driver entries in the registry, read with PowerShell. They hold
//!    the name and the real VRAM size (`HardwareInformation.qwMemorySize`); the
//!    usual `Win32_VideoController.AdapterRAM` stops at 4 GB.

use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

use super::common::{nvidia_smi_gpus, run};
use super::win_registry::parse_registry_gpus;
use crate::device::{GpuInfo, GpuVendor};

pub fn os_name() -> String {
    sysinfo::System::long_os_version().unwrap_or_else(|| "Windows".to_string())
}

pub fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = nvidia_smi_gpus();
    let have_nvidia_smi = !gpus.is_empty();
    for gpu in registry_gpus() {
        if have_nvidia_smi && gpu.vendor == GpuVendor::Nvidia {
            continue;
        }
        gpus.push(gpu);
    }
    gpus
}

// ---------- registry (display adapters) ----------

const GPU_SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*' |
  Where-Object { $_.DriverDesc } |
  ForEach-Object { [pscustomobject]@{
    name = [string]$_.DriverDesc
    id = [string]$_.MatchingDeviceId
    vram = [uint64]$_.'HardwareInformation.qwMemorySize'
  } } | ConvertTo-Json -Compress
"#;

fn registry_gpus() -> Vec<GpuInfo> {
    run("powershell", &["-NoProfile", "-NonInteractive", "-Command", GPU_SCRIPT])
        .map(|out| parse_registry_gpus(&out))
        .unwrap_or_default()
}

// ---------- processes ----------

/// Before start: no console window for llama-server.
pub fn prepare_command(cmd: &mut tokio::process::Command) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

/// One job object for the whole app. "Kill on job close": when the app ends,
/// even by a crash, Windows closes the job handle and ends every process in it.
fn job() -> Option<HANDLE> {
    // HANDLE is a raw pointer (not Send), so keep it as a number.
    static JOB: OnceLock<usize> = OnceLock::new();
    let handle = *JOB.get_or_init(|| {
        // SAFETY: plain Win32 calls with valid arguments; the handle lives for the whole app.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return 0;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(job);
                return 0;
            }
            job as usize
        }
    });
    (handle != 0).then_some(handle as HANDLE)
}

/// After start: put llama-server into the app's job.
pub fn after_spawn(child: &tokio::process::Child) -> std::io::Result<()> {
    let job = job().ok_or_else(|| std::io::Error::other("could not create a job object"))?;
    let process = child.raw_handle().ok_or_else(|| std::io::Error::other("the process has already ended"))?;
    // SAFETY: both handles are valid; the child handle is owned by `child`.
    if unsafe { AssignProcessToJobObject(job, process as HANDLE) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Windows has no gentle stop signal for a program without a window.
pub fn terminate(pid: u32) {
    kill_now(pid);
}

pub fn kill_now(pid: u32) {
    // SAFETY: the handle is checked and closed; TerminateProcess has no memory effects here.
    unsafe {
        let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !process.is_null() {
            TerminateProcess(process, 1);
            CloseHandle(process);
        }
    }
}
