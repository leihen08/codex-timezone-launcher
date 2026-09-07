use crate::timezone::validate_timezone;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settings {
    pub timezone: String,
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
    let decoded: Settings =
        serde_json::from_slice(bytes).map_err(|_| "设置文件格式无效。".to_string())?;
    Settings::new(&decoded.timezone)
}
