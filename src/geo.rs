use crate::timezone::validate_timezone;
use serde::Deserialize;

#[derive(Deserialize)]
struct IpApiResponse {
    timezone: Option<String>,
    #[serde(default)]
    error: bool,
}

pub fn parse_timezone_response(body: &[u8]) -> Result<String, String> {
    if body.len() > 64 * 1024 {
        return Err("定位服务返回的数据异常过大。".into());
    }
    let response: IpApiResponse =
        serde_json::from_slice(body).map_err(|_| "定位服务返回了无法解析的数据。".to_string())?;
    if response.error {
        return Err("定位服务暂时不可用，请稍后重试。".into());
    }
    let timezone = response
        .timezone
        .ok_or_else(|| "定位服务没有返回时区。".to_string())?;
    validate_timezone(&timezone).map_err(|_| "定位服务返回了无效时区。".into())
}
