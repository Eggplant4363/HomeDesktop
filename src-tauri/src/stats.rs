//! 系统监控（M12）：CPU 使用率（两次采样差值）+ 内存使用率
//! Windows 实现；其他平台返回 None（前端显示"不支持"）

use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    /// CPU 使用率（0-100）；首次采样无前值返回 None，下一次调用即有值
    pub cpu: Option<f64>,
    /// 内存使用率（0-100）
    pub mem: f64,
}

/// 上次 CPU 采样（idle, kernel, user 的 100ns tick）
static LAST_CPU: Mutex<Option<(u64, u64, u64)>> = Mutex::new(None);

#[tauri::command]
pub fn sys_stats() -> Option<SystemStats> {
    let cpu = cpu_usage();
    let mem = mem_usage()?;
    Some(SystemStats { cpu, mem })
}

#[cfg(target_os = "windows")]
fn cpu_usage() -> Option<f64> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;

    unsafe {
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        if GetSystemTimes(
            Some(&mut idle as *mut FILETIME),
            Some(&mut kernel as *mut FILETIME),
            Some(&mut user as *mut FILETIME),
        )
        .is_err()
        {
            return None;
        }
        let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
        let now = (ft(idle), ft(kernel), ft(user));
        let mut guard = LAST_CPU.lock().ok()?;
        let prev = guard.replace(now)?; // 首次采样无前值
        let idle_delta = now.0.saturating_sub(prev.0);
        let busy_delta = (now.1 + now.2).saturating_sub(prev.1 + prev.2);
        let total = idle_delta + busy_delta;
        if total == 0 {
            return Some(0.0);
        }
        Some(busy_delta as f64 / total as f64 * 100.0)
    }
}

#[cfg(target_os = "windows")]
fn mem_usage() -> Option<f64> {
    use std::mem::size_of;
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut status).is_err() {
            return None;
        }
        Some(status.dwMemoryLoad as f64)
    }
}

#[cfg(not(target_os = "windows"))]
fn cpu_usage() -> Option<f64> {
    None
}

#[cfg(not(target_os = "windows"))]
fn mem_usage() -> Option<f64> {
    None
}
