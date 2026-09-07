//! Process-wide single-instance guard for the interactive launcher.

use std::ptr::null;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

pub const LAUNCHER_MUTEX_NAME: &str = r"Global\ChatGPTTimeZoneLauncher-SingleInstance-v1";

pub enum AcquireResult {
    Acquired(SingleInstanceGuard),
    AlreadyRunning,
}

pub struct SingleInstanceGuard {
    handle: HANDLE,
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

pub fn acquire() -> Result<AcquireResult, String> {
    acquire_named(LAUNCHER_MUTEX_NAME)
}

pub fn acquire_named(name: &str) -> Result<AcquireResult, String> {
    if name.is_empty() || name.contains('\0') {
        return Err("单实例锁名称无效。".into());
    }

    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(format!(
            "无法创建单实例锁：{}。",
            std::io::Error::last_os_error()
        ));
    }

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        Ok(AcquireResult::AlreadyRunning)
    } else {
        Ok(AcquireResult::Acquired(SingleInstanceGuard { handle }))
    }
}
