//! OS-specific code. Every system provides the same functions:
//! `os_name`, `detect_gpus`, `prepare_command`, `after_spawn`, `terminate`, `kill_now`.

#[cfg(any(target_os = "linux", windows))]
mod common;

#[cfg(any(windows, test))]
mod win_registry;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
pub use unsupported::*;
