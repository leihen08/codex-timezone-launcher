use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command};

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

use crate::timezone::validate_timezone;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessInfo {
    pub name: String,
    pub path: Option<PathBuf>,
}

pub fn should_block_for_process(
    target: &Path,
    process_name: &str,
    process_path: Option<&Path>,
) -> bool {
    if process_name.eq_ignore_ascii_case("ChatGPT.exe") {
        return true;
    }
    if !process_name.eq_ignore_ascii_case("Codex.exe") {
        return false;
    }

    process_path.is_some_and(|running_path| {
        target
            .to_string_lossy()
            .eq_ignore_ascii_case(&running_path.to_string_lossy())
    })
}

pub fn running_processes() -> Result<Vec<ProcessInfo>, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("无法检查正在运行的客户端。".into());
    }

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut processes = Vec::new();
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        let name = utf16_buffer_to_string(&entry.szExeFile);
        if name.eq_ignore_ascii_case("ChatGPT.exe") || name.eq_ignore_ascii_case("Codex.exe") {
            processes.push(ProcessInfo {
                name,
                path: query_process_path(entry.th32ProcessID),
            });
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    Ok(processes)
}

pub fn is_client_running(target: &Path) -> Result<bool, String> {
    Ok(running_processes()?
        .iter()
        .any(|process| should_block_for_process(target, &process.name, process.path.as_deref())))
}

pub fn build_launch_command(executable: &Path, timezone: &str) -> Result<Command, String> {
    let timezone = validate_timezone(timezone)?;
    let mut command = Command::new(executable);
    command.env("TZ", timezone);
    Ok(command)
}

pub fn launch_client(executable: &Path, timezone: &str) -> Result<Child, String> {
    if !executable.is_file() {
        return Err("找到的客户端路径已经失效。".into());
    }
    build_launch_command(executable, timezone)?
        .spawn()
        .map_err(|_| "无法启动 Codex 客户端。".into())
}

fn query_process_path(process_id: u32) -> Option<PathBuf> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return None;
    }

    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let succeeded =
        unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) } != 0;
    unsafe { CloseHandle(process) };
    succeeded.then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize])))
}

fn utf16_buffer_to_string(buffer: &[u16]) -> String {
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..length])
}
