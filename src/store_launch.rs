use crate::timezone::validate_timezone;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    EVENT_MODIFY_STATE, OpenEventW, OpenThread, ResumeThread, SetEvent, THREAD_SUSPEND_RESUME,
};

pub const RESUME_EVENT_PREFIX: &str = r"Local\ChatGPTTimeZoneLauncher-";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeRequest {
    pub thread_id: u32,
    pub event_name: String,
}

pub fn build_timezone_environment(timezone: &str) -> Result<Vec<u16>, String> {
    let timezone = validate_timezone(timezone)?;
    Ok(format!("TZ={timezone}\0\0").encode_utf16().collect())
}

pub fn build_debugger_command_line(executable: &Path, event_name: &str) -> Result<String, String> {
    if !valid_event_name(event_name) {
        return Err("线程恢复事件名称无效。".into());
    }
    let executable = executable
        .to_str()
        .ok_or_else(|| "启动器路径包含无效 Unicode。".to_string())?;
    if executable.contains('"') {
        return Err("启动器路径包含不允许的字符。".into());
    }
    Ok(format!("\"{executable}\" --resume-event {event_name}"))
}

pub fn parse_resume_request<I, S>(arguments: I) -> Option<ResumeRequest>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let arguments: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    let value_after = |flag: &str| {
        arguments
            .iter()
            .position(|value| value == OsStr::new(flag))
            .and_then(|index| arguments.get(index + 1))
    };
    let event_name = value_after("--resume-event")?.to_str()?.to_string();
    let thread_id = value_after("-tid")?.to_str()?.parse::<u32>().ok()?;
    if thread_id == 0 || !valid_event_name(&event_name) {
        return None;
    }
    Some(ResumeRequest {
        thread_id,
        event_name,
    })
}

pub fn launch_store_package(
    package_full_name: &str,
    app_user_model_id: &str,
    timezone: &str,
) -> Result<(), String> {
    crate::store_activation::activate(package_full_name, app_user_model_id, timezone)
}

pub fn run_resumer_if_requested() -> bool {
    let Some(request) = parse_resume_request(std::env::args_os()) else {
        return false;
    };
    let _ = resume_package_thread(&request);
    true
}

pub fn resume_package_thread(request: &ResumeRequest) -> Result<(), String> {
    if request.thread_id == 0 || !valid_event_name(&request.event_name) {
        return Err("线程恢复请求无效。".into());
    }
    let event_name = wide(&request.event_name);
    let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr()) };
    if event.is_null() {
        return Err("无法打开启动握手事件。".into());
    }
    let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, request.thread_id) };
    if thread.is_null() {
        unsafe { CloseHandle(event) };
        return Err("无法打开待恢复线程。".into());
    }

    let resumed = unsafe { ResumeThread(thread) } != u32::MAX;
    let signaled = resumed && unsafe { SetEvent(event) } != 0;
    unsafe {
        CloseHandle(thread);
        CloseHandle(event);
    }
    if resumed && signaled {
        Ok(())
    } else {
        Err("无法恢复 Store 应用启动线程。".into())
    }
}

fn valid_event_name(value: &str) -> bool {
    value.starts_with(RESUME_EVENT_PREFIX)
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '\\' | '-'))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
