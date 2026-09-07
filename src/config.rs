use crate::timezone::validate_timezone;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settings {
    pub timezone: String,
}

#[derive(Deserialize)]
struct LegacySettings {
    #[serde(rename = "TimeZoneName")]
    time_zone_name: String,
}

impl Settings {
    pub fn new(timezone: &str) -> Result<Self, String> {
        Ok(Self {
            timezone: validate_timezone(timezone)?,
        })
    }
}

pub fn encode_settings(settings: &Settings) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(settings).map_err(|_| "无法生成设置文件。".into())
}

pub fn decode_settings(bytes: &[u8]) -> Result<Settings, String> {
    if bytes.len() > 16 * 1024 {
        return Err("设置文件异常过大。".into());
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let timezone = match serde_json::from_slice::<Settings>(bytes) {
        Ok(settings) => settings.timezone,
        Err(_) => serde_json::from_slice::<LegacySettings>(bytes)
            .map(|settings| settings.time_zone_name)
            .map_err(|_| "设置文件格式无效。".to_string())?,
    };
    Settings::new(&timezone)
}

pub fn settings_path_from_local_app_data(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("ChatGPTTimeZoneLauncher")
        .join("settings.json")
}

pub fn settings_path() -> Result<PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| settings_path_from_local_app_data(&path))
        .ok_or_else(|| "Windows 未提供本地应用数据目录。".into())
}

pub fn load_settings_at(path: &Path) -> Result<Option<Settings>, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("无法读取设置文件。".into()),
    };
    if metadata.len() > 16 * 1024 {
        return Err("设置文件异常过大。".into());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取设置文件。".to_string())?;
    decode_settings(&bytes).map(Some)
}

pub fn load_settings() -> Result<Option<Settings>, String> {
    load_settings_at(&settings_path()?)
}

pub fn save_settings_at(path: &Path, settings: &Settings) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "设置路径无效。".to_string())?;
    fs::create_dir_all(parent).map_err(|_| "无法创建设置目录。".to_string())?;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常，无法保存设置。".to_string())?
        .as_nanos();
    let temporary = parent.join(format!("settings.{}.{unique}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| "无法创建临时设置文件。".to_string())?;

    let result = (|| {
        file.write_all(&encode_settings(settings)?)
            .map_err(|_| "无法写入设置文件。".to_string())?;
        file.sync_all()
            .map_err(|_| "无法完成设置文件写入。".to_string())?;
        drop(file);
        replace_file(&temporary, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    save_settings_at(&settings_path()?, settings)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let succeeded = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded == 0 {
        Err("无法替换设置文件。".into())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(|_| "无法替换设置文件。".into())
}
