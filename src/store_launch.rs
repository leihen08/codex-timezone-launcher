use crate::timezone::validate_timezone;
use std::ffi::{OsStr, OsString};
use std::path::Path;

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

fn valid_event_name(value: &str) -> bool {
    value.starts_with(RESUME_EVENT_PREFIX)
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '\\' | '-'))
}
